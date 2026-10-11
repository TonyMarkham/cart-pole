"""Inject mass properties from FreeCAD models into exported STEP files.

FreeCAD 1.1 writes STEP geometry without material data, so downstream CAD
tools derive mass from their own default density and report wrong values.
This script runs inside FreeCAD (freecadcmd): it pairs each product in a
STEP file with its source object, computes mass = volume x density and
injects (or updates) AP214 mass and density properties directly in the
STEP text.

Usage (run from the freecad/ directory):
  freecad.cmd tools/step-mass.py -- step/base-motor.step
  freecad.cmd tools/step-mass.py -- --dry-run step/*.step
  freecad.cmd tools/step-mass.py -- --density '7200 kg/m^3' x.step
  freecad.cmd tools/step-mass.py -- --fcstd cad/x.FCStd x.step

The '--' keeps freecadcmd from interpreting our options as its own.
"""

import argparse
import os
import re
import sys

import FreeCAD as App
import Part

LINE_WIDTH = 72
VOLUME_MISMATCH_LIMIT = 0.01


class StepError(Exception):
    pass


def split_top_commas(text):
    parts, buf, depth, in_str = [], [], 0, False
    i = 0
    while i < len(text):
        c = text[i]
        if in_str:
            buf.append(c)
            if c == "'":
                if i + 1 < len(text) and text[i + 1] == "'":
                    buf.append("'")
                    i += 1
                else:
                    in_str = False
        else:
            if c == "'":
                in_str = True
                buf.append(c)
            elif c == "(":
                depth += 1
                buf.append(c)
            elif c == ")":
                depth -= 1
                buf.append(c)
            elif c == "," and depth == 0:
                parts.append("".join(buf).strip())
                buf = []
            else:
                buf.append(c)
        i += 1
    if buf:
        parts.append("".join(buf).strip())
    return parts


def split_records(region):
    chunks, buf, in_str = [], [], False
    i = 0
    while i < len(region):
        c = region[i]
        if in_str:
            buf.append(c)
            if c == "'":
                if i + 1 < len(region) and region[i + 1] == "'":
                    buf.append("'")
                    i += 1
                else:
                    in_str = False
        else:
            if c == "'":
                in_str = True
                buf.append(c)
            elif c == ";":
                chunks.append("".join(buf))
                buf = []
            else:
                buf.append(c)
        i += 1
    return [c.strip() for c in chunks if c.strip()]


def unquote(s):
    s = s.strip()
    if len(s) >= 2 and s[0] == "'" and s[-1] == "'":
        return s[1:-1].replace("''", "'")
    return None


def as_ref(s):
    s = s.strip()
    if s.startswith("#") and s[1:].isdigit():
        return int(s[1:])
    return None


def ref_list(s):
    return [as_ref(x) for x in split_top_commas(s.strip()[1:-1]) if as_ref(x)]


def step_string(s):
    return "'" + s.replace("'", "''") + "'"


def fmt_step_real(x):
    s = "%.12g" % x
    if "e" in s or "E" in s:
        mant, exp = re.split("[eE]", s)
        if "." not in mant:
            mant += ".0"
        e = int(exp)
        return "%sE%s%02d" % (mant, "+" if e >= 0 else "-", abs(e))
    if "." not in s:
        s += "."
    return s


def split_commas_any(text):
    parts, buf, in_str = [], [], False
    i = 0
    while i < len(text):
        c = text[i]
        if in_str:
            buf.append(c)
            if c == "'":
                if i + 1 < len(text) and text[i + 1] == "'":
                    buf.append("'")
                    i += 1
                else:
                    in_str = False
        elif c == "'":
            in_str = True
            buf.append(c)
        elif c == ",":
            parts.append("".join(buf))
            buf = []
        else:
            buf.append(c)
        i += 1
    if buf:
        parts.append("".join(buf))
    return parts


def fmt_record(eid, body):
    line = "#%d = %s;" % (eid, body)
    if len(line) <= LINE_WIDTH:
        return [line]
    prefix = "#%d = " % eid
    segs = split_commas_any(body)
    lines, cur = [], prefix + segs[0]
    for seg in segs[1:]:
        if len(cur) + 1 + len(seg) + 1 > LINE_WIDTH:
            lines.append(cur + ",")
            cur = "    " + seg
        else:
            cur += "," + seg
    lines.append(cur + ";")
    return lines


class Record:
    def __init__(self, rid, chunk, raw=None):
        self.id = rid
        self.raw = raw if raw is not None else chunk + ";"
        m = re.match(r"^#(\d+)\s*=\s*(.*)$", chunk, re.S)
        body = m.group(2).strip() if m else ""
        self.etype = None
        self.args = []
        self.flat = re.sub(r"\s+", " ", body)
        if not body.startswith("("):
            m2 = re.match(r"^([A-Za-z_][A-Za-z_0-9]*)\s*\((.*)\)$", body, re.S)
            if m2:
                self.etype = m2.group(1).upper()
                self.args = split_top_commas(m2.group(2))
                self.flat = re.sub(r"\s+", " ", m2.group(2).strip())


class StepFile:
    def __init__(self, path):
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            self.text = f.read()
        self.replacements = []
        self.insertions = []
        self._emitted = {}
        self._next = None
        self._parse()
        self._analyze()

    def _parse(self):
        m = re.search(r"^DATA;\s*$", self.text, re.M)
        if not m:
            raise StepError("DATA section not found")
        endsec = self.text.rfind("ENDSEC;", m.end())
        if endsec < 0:
            raise StepError("ENDSEC terminator not found")
        self.records = {}
        self.order = []
        for chunk in split_records(self.text[m.end():endsec]):
            m2 = re.match(r"^#(\d+)\s*=", chunk)
            if not m2:
                continue
            rid = int(m2.group(1))
            self.records[rid] = Record(rid, chunk)
            self.order.append(rid)

    def _analyze(self):
        self.product_name = {}
        self.formation_product = {}
        self.def_formation = {}
        self.pds_def = {}
        self.sdr_rep = {}
        self.rep_ctx = {}
        self.contexts = []
        self.pd_pds = {}
        self.mass_props = {}
        self.density_props = {}
        self.derived_units = {}
        self.instance_counts = {}
        self.pds_with_sdr = set()
        for rid in self.order:
            rec = self.records[rid]
            a, t = rec.args, rec.etype
            if t == "PRODUCT" and a:
                self.product_name[rid] = unquote(a[0])
            elif t == "PRODUCT_DEFINITION_FORMATION" and len(a) >= 3:
                self.formation_product[rid] = as_ref(a[2])
            elif t == "PRODUCT_DEFINITION" and len(a) >= 3:
                self.def_formation[rid] = as_ref(a[2])
            elif t == "PRODUCT_DEFINITION_SHAPE" and len(a) >= 3:
                self.pds_def[rid] = as_ref(a[2])
            elif t == "SHAPE_DEFINITION_REPRESENTATION" and len(a) >= 2:
                self.sdr_rep[as_ref(a[1])] = as_ref(a[0])
                pds = as_ref(a[0])
                if pds is not None:
                    self.pds_with_sdr.add(pds)
            elif t == "NEXT_ASSEMBLY_USAGE_OCCURRENCE" and len(a) >= 5:
                child_pd = as_ref(a[4])
                formation = self.def_formation.get(child_pd) if child_pd else None
                product = self.formation_product.get(formation) if formation else None
                if product is not None:
                    self.instance_counts[product] = (
                        self.instance_counts.get(product, 0) + 1
                    )
            elif t and t.endswith("_SHAPE_REPRESENTATION") and a:
                self.rep_ctx[rid] = as_ref(a[-1])
            if "GLOBAL_UNIT_ASSIGNED_CONTEXT" in rec.flat:
                self.contexts.append(rid)
        for pdsid, pdid in self.pds_def.items():
            self.pd_pds[pdid] = pdsid
        for rid in self.order:
            rec = self.records[rid]
            if rec.etype == "PROPERTY_DEFINITION" and len(rec.args) >= 3:
                desc = (unquote(rec.args[1]) or "").upper()
                pds = as_ref(rec.args[2])
                if pds is None:
                    continue
                if pds not in self.pds_def and pds in self.pd_pds:
                    pds = self.pd_pds[pds]
                if desc == "MASS" and pds in self.pds_def:
                    self.mass_props.setdefault(pds, []).append(rid)
                elif desc in ("DENSITY", "MATERIAL NAME") and pds in self.pds_def:
                    self.density_props.setdefault(pds, []).append(rid)
            elif rec.etype == "DERIVED_UNIT" and len(rec.args) == 1:
                comp = []
                for eid in ref_list(rec.args[0]):
                    erec = self.records.get(eid)
                    if (
                        erec is None
                        or erec.etype != "DERIVED_UNIT_ELEMENT"
                        or len(erec.args) < 2
                    ):
                        comp = []
                        break
                    uid = as_ref(erec.args[0])
                    try:
                        comp.append((uid, float(erec.args[1])))
                    except ValueError:
                        comp = []
                        break
                if comp:
                    self.derived_units.setdefault(tuple(sorted(comp)), []).append(rid)

    def products(self):
        return [
            (rid, self.product_name[rid])
            for rid in self.order
            if rid in self.product_name
        ]

    def pds_for_product(self, pid):
        pds_list = []
        for fid, fprod in self.formation_product.items():
            if fprod != pid:
                continue
            for pdid, pform in self.def_formation.items():
                if pform != fid:
                    continue
                pds_list.extend(
                    pdsid for pdsid, pdef in self.pds_def.items() if pdef == pdid
                )
        return pds_list

    def context_for_pds(self, pds_id):
        for rep, pds in self.sdr_rep.items():
            if pds == pds_id and rep in self.rep_ctx:
                return self.rep_ctx[rep]
        if len(self.contexts) == 1:
            return self.contexts[0]
        raise StepError("no representation context found for product")

    def context_units(self, ctx_id):
        m = re.search(
            r"GLOBAL_UNIT_ASSIGNED_CONTEXT\s*\(\s*\(\s*([#0-9,\s]+?)\s*\)\s*\)",
            self.records[ctx_id].raw,
        )
        if not m:
            return None
        return [int(x) for x in re.findall(r"\d+", m.group(1))], m

    def is_mass_unit(self, uid):
        rec = self.records.get(uid)
        if rec is None:
            return False
        return (rec.etype is None and "MASS_UNIT" in rec.flat) or rec.etype == "MASS_UNIT"

    def ensure_mass_unit(self, ctx_id):
        found = self.context_units(ctx_id)
        if found:
            for uid in found[0]:
                if self.is_mass_unit(uid):
                    return uid
        for rid in self.order:
            if self.is_mass_unit(rid):
                self._add_unit_to_context(ctx_id, rid)
                return rid
        uid = self._new_record("( MASS_UNIT() NAMED_UNIT(*) SI_UNIT(.KILO.,.GRAM.) )")
        if not self._add_unit_to_context(ctx_id, uid):
            self._new_mass_context(ctx_id, uid)
        return uid

    def _add_unit_to_context(self, ctx_id, uid):
        found = self.context_units(ctx_id)
        if not found:
            return False
        units, m = found
        if uid in units:
            return True
        rec = self.records[ctx_id]
        new_ids = ",".join("#%d" % u for u in units + [uid])
        new_raw = (
            rec.raw[: m.start()]
            + "GLOBAL_UNIT_ASSIGNED_CONTEXT((" + new_ids + "))"
            + rec.raw[m.end():]
        )
        self.replacements.append((rec.raw, new_raw))
        rec.raw = new_raw
        return True

    def _new_mass_context(self, ctx_id, mass_uid):
        found = self.context_units(ctx_id)
        if not found:
            raise StepError("cannot build fallback unit context")
        units = found[0]
        length = [u for u in units if "LENGTH_UNIT" in self.records[u].flat]
        angle = [u for u in units if "PLANE_ANGLE_UNIT" in self.records[u].flat]
        if not length or not angle:
            raise StepError("cannot build fallback unit context")
        body = (
            "( GEOMETRIC_REPRESENTATION_CONTEXT(3) "
            "GLOBAL_UNIT_ASSIGNED_CONTEXT((#%d,#%d,#%d)) REPRESENTATION_CONTEXT"
            " ('Context #1','3D Context with UNIT and UNCERTAINTY') )"
            % (length[0], angle[0], mass_uid)
        )
        return self._new_record(body)

    def next_id(self):
        if self._next is None:
            self._next = max(self.records) + 1 if self.records else 1
        while self._next in self.records:
            self._next += 1
        nid = self._next
        self._next += 1
        return nid

    def _emit(self, eid, body):
        lines = fmt_record(eid, body)
        self.insertions.extend(lines)
        self._emitted[eid] = lines
        self.records[eid] = Record(eid, "#%d = %s" % (eid, body), raw="\n".join(lines))
        self.order.append(eid)
        return eid

    def _new_record(self, body):
        return self._emit(self.next_id(), body)

    def inject_mass(self, pds_id, mass_kg):
        ctx_id = self.context_for_pds(pds_id)
        unit_id = self.ensure_mass_unit(ctx_id)
        e_prop = self.next_id()
        e_rep = self.next_id()
        e_item = self.next_id()
        self._emit(e_prop, "PROPERTY_DEFINITION('mass property','MASS',#%d)" % pds_id)
        self._emit(
            self.next_id(),
            "PROPERTY_DEFINITION_REPRESENTATION(#%d,#%d)" % (e_prop, e_rep),
        )
        self._emit(
            e_rep, "REPRESENTATION('mass property',(#%d),#%d)" % (e_item, ctx_id)
        )
        self._emit(
            e_item,
            "MEASURE_REPRESENTATION_ITEM('MASS',MASS_MEASURE(%s),#%d)"
            % (fmt_step_real(mass_kg), unit_id),
        )

    def _property_items(self):
        pdr_map = {}
        pdr_ids = {}
        rep_items = {}
        for rid in self.order:
            rec = self.records[rid]
            if rec.etype == "PROPERTY_DEFINITION_REPRESENTATION" and len(rec.args) >= 2:
                prop = as_ref(rec.args[0])
                pdr_map[prop] = as_ref(rec.args[1])
                pdr_ids[prop] = rid
            elif rec.etype == "REPRESENTATION" and len(rec.args) >= 2:
                rep_items[rid] = ref_list(rec.args[1])
        return pdr_map, pdr_ids, rep_items

    def update_mass(self, pds_id, mass_kg):
        prop_ids = self.mass_props.get(pds_id)
        if not prop_ids:
            return False
        pdr_map, pdr_ids, rep_items = self._property_items()
        fresh = True
        for prop_id in prop_ids:
            for iid in rep_items.get(pdr_map.get(prop_id), []):
                rec = self.records.get(iid)
                if rec is None or not re.sub(
                    r"\s+", "", rec.flat
                ).startswith("'MASS',MASS_MEASURE("):
                    fresh = False
        if not fresh:
            doomed = set()
            for prop_id in prop_ids:
                doomed.add(prop_id)
                doomed.add(pdr_ids.get(prop_id))
                doomed.add(pdr_map.get(prop_id))
                doomed.update(rep_items.get(pdr_map.get(prop_id), []))
            for rid in doomed:
                if rid is not None:
                    self._delete_record(rid)
            return False
        updated = False
        for prop_id in prop_ids:
            for iid in rep_items.get(pdr_map.get(prop_id), []):
                rec = self.records.get(iid)
                if rec is None or "MASS_MEASURE" not in rec.flat:
                    continue
                new_raw, nsub = re.subn(
                    r"(MASS_MEASURE\s*\(\s*)[-+0-9.eE]+(\s*\))",
                    lambda mm, v=mass_kg: mm.group(1) + fmt_step_real(v) + mm.group(2),
                    rec.raw,
                    count=1,
                )
                if nsub:
                    if new_raw != rec.raw:
                        self.replacements.append((rec.raw, new_raw))
                        rec.raw = new_raw
                    updated = True
        return updated

    def _unit_by_composition(self, markers):
        for rid in self.order:
            rec = self.records.get(rid)
            if rec is None:
                continue
            flat = rec.flat.replace(" ", "")
            if all(m in flat for m in markers) and (
                rec.etype is None or rec.etype.endswith("_UNIT")
            ):
                return rid
        return None

    def _density_unit(self):
        gun = self._unit_by_composition(("MASS_UNIT", "SI_UNIT($,.GRAM.)"))
        if gun is None:
            gun = self._new_record("( MASS_UNIT() NAMED_UNIT(*) SI_UNIT($,.GRAM.) )")
        cmu = self._unit_by_composition(("LENGTH_UNIT", "SI_UNIT(.CENTI.,.METRE.)"))
        if cmu is None:
            cmu = self._new_record(
                "( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.CENTI.,.METRE.) )"
            )
        key = tuple(sorted(((gun, 3.0), (cmu, 2.0))))
        existing = self.derived_units.get(key)
        if existing:
            return existing[0]
        e1 = self._new_record("DERIVED_UNIT_ELEMENT(#%d,3.)" % gun)
        e2 = self._new_record("DERIVED_UNIT_ELEMENT(#%d,2.)" % cmu)
        du = self._new_record("DERIVED_UNIT((#%d,#%d))" % (e1, e2))
        self.derived_units.setdefault(key, []).append(du)
        return du

    def inject_density(self, pds_id, density_kg_per_mm3, material_name):
        pd_id = self.pds_def.get(pds_id)
        if pd_id is None:
            raise StepError("no product definition found for density property")
        ctx_id = self.context_for_pds(pds_id)
        du = self._density_unit()
        value = density_kg_per_mm3 * 1e6
        e_dri = self.next_id()
        e_p1 = self.next_id()
        e_r1 = self.next_id()
        e_mri = self.next_id()
        e_p2 = self.next_id()
        e_r2 = self.next_id()
        self._emit(
            e_dri,
            "DESCRIPTIVE_REPRESENTATION_ITEM(%s,%s)"
            % (step_string(material_name), step_string(material_name)),
        )
        self._emit(e_r1, "REPRESENTATION('material name',(#%d),#%d)" % (e_dri, ctx_id))
        self._emit(
            e_p1, "PROPERTY_DEFINITION('material property','material name',#%d)" % pd_id
        )
        self._emit(
            self.next_id(),
            "PROPERTY_DEFINITION_REPRESENTATION(#%d,#%d)" % (e_p1, e_r1),
        )
        self._emit(
            e_mri,
            "MEASURE_REPRESENTATION_ITEM('DENSITY',RATIO_MEASURE(%s),#%d)"
            % (fmt_step_real(value), du),
        )
        self._emit(e_r2, "REPRESENTATION('density',(#%d),#%d)" % (e_mri, ctx_id))
        self._emit(
            e_p2, "PROPERTY_DEFINITION('material property','density',#%d)" % pd_id
        )
        self._emit(
            self.next_id(),
            "PROPERTY_DEFINITION_REPRESENTATION(#%d,#%d)" % (e_p2, e_r2),
        )

    def _density_chain_items(self, prop_id, pdr_map, rep_items):
        rep_id = pdr_map.get(prop_id)
        items = rep_items.get(rep_id, [])
        has_dri = False
        mri_ok = False
        for iid in items:
            irec = self.records.get(iid)
            if irec is None:
                continue
            if irec.etype == "DESCRIPTIVE_REPRESENTATION_ITEM":
                has_dri = True
            elif "RATIO_MEASURE" in irec.flat:
                unit_ref = as_ref(irec.args[-1]) if irec.args else None
                du = self.records.get(unit_ref) if unit_ref else None
                if du is None or du.etype != "DERIVED_UNIT":
                    continue
                ok = True
                for eid in ref_list(du.args[0]):
                    erec = self.records.get(eid)
                    if (
                        erec is None
                        or erec.etype != "DERIVED_UNIT_ELEMENT"
                        or len(erec.args) < 2
                    ):
                        ok = False
                        break
                    urec = self.records.get(as_ref(erec.args[0]) or 0)
                    if urec is None:
                        ok = False
                        break
                    uflat = urec.flat.replace(" ", "")
                    if "MASS_UNIT" in uflat and "SI_UNIT($,.GRAM.)" in uflat:
                        continue
                    if "LENGTH_UNIT" in uflat and "SI_UNIT(.CENTI.,.METRE.)" in uflat:
                        continue
                    ok = False
                    break
                mri_ok = ok
        return has_dri, mri_ok, items

    def update_density(self, pds_id, density_kg_per_mm3, material_name):
        prop_ids = self.density_props.get(pds_id)
        if not prop_ids:
            return False
        pdr_map, pdr_ids, rep_items = self._property_items()
        has_dri = False
        has_mri = False
        fresh = True
        for prop_id in prop_ids:
            rec = self.records.get(prop_id)
            target = as_ref(rec.args[2]) if rec and len(rec.args) >= 3 else None
            if target is None or target not in self.pd_pds:
                fresh = False
                continue
            dri, mri, _ = self._density_chain_items(prop_id, pdr_map, rep_items)
            has_dri = has_dri or dri
            has_mri = has_mri or mri
        if not fresh or not has_dri or not has_mri:
            doomed = set()
            for prop_id in prop_ids:
                doomed.add(prop_id)
                doomed.add(pdr_ids.get(prop_id))
                doomed.add(pdr_map.get(prop_id))
                doomed.update(rep_items.get(pdr_map.get(prop_id), []))
            for rid in doomed:
                if rid is not None:
                    self._delete_record(rid)
            return False
        value = density_kg_per_mm3 * 1e6
        updated = False
        for prop_id in prop_ids:
            for iid in rep_items.get(pdr_map.get(prop_id), []):
                irec = self.records.get(iid)
                if irec is None:
                    continue
                if irec.etype == "DESCRIPTIVE_REPRESENTATION_ITEM":
                    new_raw, nsub = re.subn(
                        r"(DESCRIPTIVE_REPRESENTATION_ITEM\s*\(\s*)'(?:[^']|'')*'",
                        lambda mm, n=material_name: mm.group(1) + step_string(n),
                        irec.raw,
                        count=1,
                    )
                elif "RATIO_MEASURE" in irec.flat:
                    new_raw, nsub = re.subn(
                        r"(RATIO_MEASURE\s*\(\s*)[-+0-9.eE]+(\s*\))",
                        lambda mm, v=value: mm.group(1) + fmt_step_real(v) + mm.group(2),
                        irec.raw,
                        count=1,
                    )
                else:
                    continue
                if nsub:
                    if new_raw != irec.raw:
                        self.replacements.append((irec.raw, new_raw))
                        irec.raw = new_raw
                    updated = True
        return updated

    def _delete_record(self, rid):
        rec = self.records.get(rid)
        if rec is None:
            return
        if rid in self._emitted:
            for line in self._emitted[rid]:
                if line in self.insertions:
                    self.insertions.remove(line)
        else:
            idx = self.text.find(rec.raw)
            if idx >= 0:
                end = idx + len(rec.raw)
                old = rec.raw + (
                    "\n" if end < len(self.text) and self.text[end] == "\n" else ""
                )
                self.replacements.append((old, ""))
        self._emitted.pop(rid, None)
        self.records.pop(rid, None)
        if rid in self.order:
            self.order.remove(rid)

    def _purge_orphans(self):
        while True:
            refcount = {}
            for rid in self.order:
                rec = self.records.get(rid)
                if rec is None:
                    continue
                for x in re.findall(r"#(\d+)", rec.raw):
                    refcount[int(x)] = refcount.get(int(x), 0) + 1
            doomed = [
                rid
                for rid in self.order
                if rid in self.records
                and self.records[rid].etype in ("DERIVED_UNIT", "DERIVED_UNIT_ELEMENT")
                and refcount.get(rid, 0) <= 1
            ]
            for rid in self.order:
                rec = self.records.get(rid)
                if rec is None or rec.etype != "PROPERTY_DEFINITION_REPRESENTATION":
                    continue
                prop = as_ref(rec.args[0]) if len(rec.args) >= 1 else None
                rep = as_ref(rec.args[1]) if len(rec.args) >= 2 else None
                if prop not in self.records or rep not in self.records:
                    doomed.append(rid)
            if not doomed:
                return
            for rid in doomed:
                self._delete_record(rid)

    def apply(self):
        self._purge_orphans()
        text = self.text
        for old, new in self.replacements:
            if text.count(old) != 1:
                raise StepError("record no longer unique, aborting")
            text = text.replace(old, new, 1)
        if self.insertions:
            endsec = text.rfind("ENDSEC;")
            head = text[:endsec]
            block = "\n".join(self.insertions) + "\n"
            if head and not head.endswith("\n"):
                block = "\n" + block
            text = head + block + text[endsec:]
        return text


def density_to_kg_per_mm3(text):
    q = App.Units.Quantity(text)
    v = q.getValueAs("kg/mm^3")
    if hasattr(v, "Value"):
        v = v.Value
    return float(v)


def material_density(obj):
    mat = getattr(obj, "ShapeMaterial", None)
    if mat is not None:
        props = getattr(mat, "PhysicalProperties", None)
        if isinstance(props, dict):
            s = props.get("Density")
            if s:
                return density_to_kg_per_mm3(s), str(getattr(mat, "Name", "material"))
    m = getattr(obj, "Material", None)
    if isinstance(m, dict) and m.get("Density"):
        return density_to_kg_per_mm3(m["Density"]), "material dict"
    return None, None


def collect_doc_objects(doc, out, opened, visited):
    for obj in doc.Objects:
        shape = getattr(obj, "Shape", None)
        if shape is None or shape.isNull() or not shape.Solids:
            continue
        if shape.Volume <= 0:
            continue
        dens, mat_name = material_density(obj)
        out.append(
            {
                "label": obj.Label,
                "name": obj.Name,
                "volume": shape.Volume,
                "density": dens,
                "material": mat_name,
            }
        )
        lo = getattr(obj, "LinkedObject", None)
        if lo is None:
            continue
        target = lo.Document
        if target.Name in visited:
            continue
        visited.add(target.Name)
        opened.append(target)
        collect_doc_objects(target, out, opened, visited)


def doc_candidates(doc_path):
    doc = App.openDocument(doc_path)
    opened = [doc]
    try:
        out = []
        collect_doc_objects(doc, out, opened, {doc.Name})
        return out
    finally:
        for d in opened:
            App.closeDocument(d.Name)


def match_candidate(cands, name):
    exact = [c for c in cands if c["label"] == name]
    if not exact:
        exact = [c for c in cands if c["name"] == name]
    if not exact and len(cands) == 1:
        exact = cands
    return exact[0] if exact else None


def resolve_fcstd(step_path, args):
    if args.fcstd:
        return args.fcstd
    step_dir = os.path.dirname(os.path.abspath(step_path))
    stem = os.path.splitext(os.path.basename(step_path))[0]
    for cand in (
        os.path.join(step_dir, "..", "cad", stem + ".FCStd"),
        os.path.join(step_dir, stem + ".FCStd"),
    ):
        if os.path.isfile(cand):
            return cand
    return None


def process_step(path, args):
    print("== %s" % path)
    sf = StepFile(path)
    products = sf.products()
    if not products:
        raise StepError("no PRODUCT entities found in %s" % path)
    fcstd = resolve_fcstd(path, args)
    cands = []
    if fcstd:
        print("source model: %s" % fcstd)
        cands = doc_candidates(fcstd)
    elif not args.density:
        raise StepError(
            "no matching FCStd found for %s; pass --fcstd or --density" % path
        )
    override = density_to_kg_per_mm3(args.density) if args.density else None
    expected_total = 0.0
    all_known = True
    has_instances = bool(sf.instance_counts)
    plan = []
    for pid, name in products:
        pds_list = sf.pds_for_product(pid)
        if not pds_list:
            raise StepError("PRODUCT '%s' has no product definition shape" % name)
        pds_id = next(
            (p for p in pds_list if p in sf.pds_with_sdr), pds_list[0]
        )
        cand = match_candidate(cands, name)
        volume = cand["volume"] if cand else None
        density, material = override, "--density override"
        if cand and density is None:
            density, material = cand["density"], cand["material"]
        if volume is None and len(products) == 1:
            volume = Part.read(path).Volume
        instances = sf.instance_counts.get(pid, 0) if has_instances else 1
        if density is None:
            if len(products) == 1:
                raise StepError(
                    "object '%s' has no material density; assign a material in "
                    "FreeCAD or pass --density" % name
                )
            print(
                "  %-20s no material assigned - skipped%s"
                % (name, " (assembly container)" if has_instances and instances == 0 else "")
            )
            if not (has_instances and instances == 0):
                all_known = False
            continue
        if volume is None:
            all_known = False
            print(
                "  %-20s no source object found and no volume available - skipped"
                % name
            )
            continue
        mass = volume * density
        mat_label = (cand["material"] if cand else None) or material
        suffix = "" if instances == 1 else "  x%d instances" % instances
        print(
            "  %-20s vol=%12.4f mm^3  dens=%.6g kg/mm^3 = %.6g g/cm^3 (%s)%s"
            "  ->  mass=%.9g kg"
            % (name, volume, density, density * 1e6, material, suffix, mass)
        )
        plan.append((pds_id, mass, density, mat_label))
        expected_total += volume * instances
    if args.dry_run:
        print("dry run, no changes written")
        return None
    if not plan:
        raise StepError("no products could be patched - nothing written")
    if fcstd and expected_total:
        imported = Part.read(path).Volume
        rel = abs(imported - expected_total) / max(expected_total, 1e-12)
        if rel > VOLUME_MISMATCH_LIMIT:
            raise StepError(
                "STEP geometry volume %.4f mm^3 differs from source model "
                "volume %.4f mm^3 (%.2f%%) - re-export the STEP file "
                "before patching mass" % (imported, expected_total, rel * 100)
            )
    for pds_id, mass, density, mat_label in plan:
        if sf.update_mass(pds_id, mass):
            print("  updated existing mass property of #%d" % pds_id)
        else:
            sf.inject_mass(pds_id, mass)
            print("  injected mass property for #%d" % pds_id)
        if sf.update_density(pds_id, density, mat_label):
            print("  updated existing density property of #%d" % pds_id)
        else:
            sf.inject_density(pds_id, density, mat_label)
            print("  injected density property for #%d" % pds_id)
    out = args.out or path
    with open(out, "w", encoding="utf-8") as f:
        f.write(sf.apply())
    print("wrote %s" % out)
    return expected_total if all_known else None


def verify(path, expected_volume):
    shape = Part.read(path)
    vol = shape.Volume
    if expected_volume is None:
        print("verify: import ok (volume %.4f mm^3)" % vol)
        return
    rel = abs(vol - expected_volume) / max(expected_volume, 1e-12)
    if rel > VOLUME_MISMATCH_LIMIT:
        raise StepError(
            "verification failed: STEP volume %.4f mm^3 differs from source "
            "model volume %.4f mm^3 (%.2f%%) - re-export the STEP file"
            % (vol, expected_volume, rel * 100)
        )
    print("verify: import ok, volume %.4f mm^3 matches source model" % vol)


def main(argv):
    p = argparse.ArgumentParser(
        description="Inject mass properties from FreeCAD models into STEP files."
    )
    p.add_argument("steps", nargs="+", metavar="STEP", help="STEP file(s) to patch")
    p.add_argument(
        "--fcstd", help="source FreeCAD document (default: cad/<step-stem>.FCStd)"
    )
    p.add_argument("--density", help="density override, e.g. '7200 kg/m^3'")
    p.add_argument(
        "--out", help="write to this path instead of in place (single STEP only)"
    )
    p.add_argument(
        "--dry-run", action="store_true", help="print computed masses, write nothing"
    )
    p.add_argument(
        "--no-verify", action="store_true", help="skip re-import check after writing"
    )
    args = p.parse_args(argv)
    if args.out and len(args.steps) > 1:
        p.error("--out works with a single STEP input")
    ok = True
    for path in args.steps:
        try:
            expected = process_step(path, args)
            if not args.dry_run and not args.no_verify:
                verify(args.out or path, expected)
        except Exception as exc:
            print("ERROR: %s: %s" % (path, exc))
            ok = False
    return 0 if ok else 1


argv = sys.argv
if argv and os.path.basename(argv[0]).lower().startswith("freecad"):
    argv = argv[1:]
if argv and argv[0].endswith(".py"):
    argv = argv[1:]
argv = [a for a in argv if a not in ("--pass", "--")]
code = main(argv)
sys.stdout.flush()
sys.stderr.flush()
os._exit(code)