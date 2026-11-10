# Builds a 4-foot aluminium drywall T-square. Every marking - the blade's
# two scales, the stud diamonds, the head's scale and lettering - is a
# base-map texture on the top faces, not geometry.
# Run: blender --background --factory-startup --python build_square.py -- <out_dir> <preview_dir>
#   <out_dir> is the Tools folder: it writes blender~/drywall-square.blend,
#   fbx/drywall-square.fbx and Textures/drywall-square-marks.png there.
#
# Coordinates (inches here, metres in the model): the blade runs along +X
# from the head's inner face (x = 0) to its end (x = 48); y = 0 is the
# blade's centreline; z = 0 is the board the square lies on.
import math
import os
import sys

import bmesh
import bpy
import numpy as np
from mathutils import Vector

IN = 0.0254  # metres per inch

# Dimensions (inches)
BLADE_REACH = 48.0      # measured length, head's inner face to blade end
BLADE_WIDTH = 2.0
BLADE_THICK = 0.125
HEAD_WIDTH = 1.5        # along x
HEAD_THICK = 0.375
HEAD_Y0, HEAD_Y1 = -15.25, 6.75   # the head sits off-centre on the blade

LINE_W = 0.016          # tick line width (~0.4 mm)
# Tick length by graduation: inch, 1/2, 1/4, 1/8, 1/16.
TICK_LEN = {1: 0.38, 2: 0.27, 4: 0.20, 8: 0.15, 16: 0.10}
NUMBER_SIZE = 0.26
NUMBER_INSET = 0.62     # number centre's distance from its scale's edge
STUD_MARKS = (16.0, 32.0)

# Texture layout: each strip of a top face renders into its own band of
# one TEX x TEX image at the same density, so Unity's default 2048
# import size keeps it. Bands keep MARGIN px of plain aluminium around
# each strip so neighbours never bleed into each other.
TEX = 2048
MARGIN = 16
BLADE_ROWS = 4
HEAD_ROWS = 2

ALU = (0.91, 0.92, 0.92)
INK = (0.015, 0.015, 0.015)

argv = sys.argv[sys.argv.index("--") + 1:]
out_dir, preview_dir = argv[0], argv[1]
tex_dir = os.path.join(out_dir, "Textures")
fbx_dir = os.path.join(out_dir, "fbx")
blend_dir = os.path.join(out_dir, "blender~")  # "~": Unity ignores the folder
for d in (tex_dir, fbx_dir, blend_dir, preview_dir):
    os.makedirs(d, exist_ok=True)
tex_path = os.path.join(tex_dir, "drywall-square-marks.png")

X0 = -HEAD_WIDTH                         # blade's start, over the head
blade_seg = (BLADE_REACH - X0) / BLADE_ROWS
head_seg = (HEAD_Y1 - HEAD_Y0) / HEAD_ROWS
PPI = (TEX - 2 * MARGIN) / max(blade_seg, head_seg)   # pixels per inch


class Strip:
    """A rectangle of a top face and the texture band it renders into.
    axis_u / axis_v are the world directions of the image's right / up."""

    def __init__(self, centre, across, axis_u, axis_v, v0):
        self.centre = centre
        self.axis_u = axis_u
        self.axis_v = axis_v
        self.height = int(math.ceil(across * PPI)) + 2 * MARGIN
        self.v0 = v0

    def uv(self, x, y):
        d = (x - self.centre[0], y - self.centre[1])
        u = TEX / 2 + (d[0] * self.axis_u[0] + d[1] * self.axis_u[1]) * PPI
        v = self.v0 + self.height / 2 + (d[0] * self.axis_v[0] + d[1] * self.axis_v[1]) * PPI
        return u / TEX, v / TEX


v_cursor = 0
blade_strips = []
for row in range(BLADE_ROWS):
    cx = X0 + (row + 0.5) * blade_seg
    s = Strip((cx, 0.0), BLADE_WIDTH, (1, 0), (0, 1), v_cursor)
    blade_strips.append(s)
    v_cursor += s.height
head_strips = []
for row in range(HEAD_ROWS):
    cy = HEAD_Y0 + (row + 0.5) * head_seg
    # Long axis along y: image right = +y, image up = -x.
    s = Strip((-HEAD_WIDTH / 2, cy), HEAD_WIDTH, (0, 1), (-1, 0), v_cursor)
    head_strips.append(s)
    v_cursor += s.height
strips = blade_strips + head_strips
assert v_cursor <= TEX, f"bands need {v_cursor} px"


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.preferences.filepaths.save_version = 0
    s = bpy.context.scene
    s.unit_settings.system = 'METRIC'
    s.unit_settings.scale_length = 1.0
    return s


def material(name, color, metallic, roughness):
    mat = bpy.data.materials.new(name)
    try:
        mat.use_nodes = True
    except Exception:
        pass
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    mat.diffuse_color = (*color, 1.0)
    mat.metallic = metallic
    mat.roughness = roughness
    return mat


def look_at(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat('-Z', 'Y').to_euler()


# ---- Stage 1: lay the markings out flat and render them into the texture ----
scene = reset()
ink = material("Ink", INK, 0.0, 1.0)
alu = material("AluFlat", ALU, 0.0, 1.0)

bm = bmesh.new()


def quad(points):
    bm.faces.new([bm.verts.new((x * IN, y * IN, 0.0)) for x, y in points])


def rect(x0, x1, y0, y1):
    quad(((x0, y0), (x1, y0), (x1, y1), (x0, y1)))


def graduation(k):
    """Sixteenths index -> the coarsest graduation it falls on."""
    for g in (1, 2, 4, 8):
        if k % (16 // g) == 0:
            return g
    return 16


def text(body, x, y, size, rot_deg):
    curve = bpy.data.curves.new(body, 'FONT')
    curve.body = body
    curve.size = size * IN
    curve.align_x = 'CENTER'
    curve.align_y = 'CENTER'
    curve.offset = 0.004 * IN          # slightly bolder strokes
    obj = bpy.data.objects.new(body, curve)
    obj.location = (x * IN, y * IN, 0.001)
    obj.rotation_euler = (0, 0, math.radians(rot_deg))
    scene.collection.objects.link(obj)
    obj.data.materials.append(ink)


half_w = BLADE_WIDTH / 2

# Blade: a 1/16" scale on each long edge. The +y edge counts 1..47 from
# the head; the -y edge counts 1..47 from the blade's end. Each scale's
# numbers sit inboard of its ticks, their tops toward their own edge.
for k in range(1, int(BLADE_REACH * 16)):
    x = k / 16.0
    length = TICK_LEN[graduation(k)]
    rect(x - LINE_W / 2, x + LINE_W / 2, half_w - length, half_w)
    rect(x - LINE_W / 2, x + LINE_W / 2, -half_w, -half_w + length)
for n in range(1, int(BLADE_REACH)):
    text(str(n), n, half_w - NUMBER_INSET, NUMBER_SIZE, 0)
    text(str(n), BLADE_REACH - n, -half_w + NUMBER_INSET, NUMBER_SIZE, 180)

# Stud marks (16" on centre) on the centreline, and the end note.
for xs in STUD_MARKS:
    quad(((xs - 0.16, 0.0), (xs, -0.26), (xs + 0.16, 0.0), (xs, 0.26)))
text('ALLOW 1/8" AT END', 24.0, 0.0, 0.10, 0)

# Head: a 1/16" scale along its inner edge (x = 0), measured from the
# blade's centreline both ways; numbers and title read from the blade's
# end, so their tops face the head's outer edge (-x). Graduations and
# numbers under the blade are left out.
k_lo = int(math.ceil(HEAD_Y0 * 16)) + 1
k_hi = int(math.floor(HEAD_Y1 * 16))
for k in range(k_lo, k_hi):
    if abs(k) <= 16 * half_w:
        continue
    y = k / 16.0
    length = TICK_LEN[graduation(abs(k))]
    rect(-length, 0.0, y - LINE_W / 2, y + LINE_W / 2)
for n in range(2, int(-HEAD_Y0) + 1):
    text(str(n), -0.62, -n, NUMBER_SIZE, 90)
for n in range(2, int(HEAD_Y1)):
    text(str(n), -0.62, n, NUMBER_SIZE, 90)
text('48" DRYWALL T-SQUARE', -1.18, -5.0, 0.17, 90)

marks = bpy.data.meshes.new("Marks")
bm.to_mesh(marks)
bm.free()
marks_obj = bpy.data.objects.new("Marks", marks)
marks_obj.location.z = 0.001
scene.collection.objects.link(marks_obj)
marks_obj.data.materials.append(ink)

bpy.ops.mesh.primitive_plane_add(size=1.0, location=(0.6, -0.1, 0))
backing = bpy.context.active_object
backing.scale = (3.0, 1.5, 1.0)
backing.data.materials.append(alu)

scene.render.engine = 'BLENDER_WORKBENCH'
scene.display.shading.light = 'FLAT'
scene.display.shading.color_type = 'MATERIAL'
scene.display.shading.show_shadows = False
scene.display.shading.show_cavity = False
scene.display.shading.show_specular_highlight = False
scene.display.render_aa = '32'
scene.view_settings.view_transform = 'Standard'
scene.view_settings.look = 'None'
scene.render.resolution_x = TEX
scene.render.image_settings.file_format = 'PNG'
scene.render.image_settings.color_mode = 'RGB'

cam = bpy.data.objects.new("Cam", bpy.data.cameras.new("Cam"))
cam.data.type = 'ORTHO'
cam.data.sensor_fit = 'HORIZONTAL'
cam.data.ortho_scale = (TEX / PPI) * IN
scene.collection.objects.link(cam)
scene.camera = cam

atlas = None
for i, s in enumerate(strips):
    scene.render.resolution_y = s.height
    cam.location = (s.centre[0] * IN, s.centre[1] * IN, 1.0)
    cam.rotation_euler = (0.0, 0.0, math.atan2(s.axis_u[1], s.axis_u[0]))
    tile_path = os.path.join(preview_dir, f"tile{i}.png")
    scene.render.filepath = tile_path
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(tile_path)
    px = np.array(img.pixels[:], dtype=np.float32).reshape(s.height, TEX, 4)
    if atlas is None:
        atlas = np.empty((TEX, TEX, 4), dtype=np.float32)
        atlas[:] = px[0, 0]                 # plain aluminium everywhere else
    atlas[s.v0:s.v0 + s.height] = px        # rows bottom-up, like UV v
    bpy.data.images.remove(img)

out_img = bpy.data.images.new("drywall-square-marks", TEX, TEX, alpha=False)
out_img.colorspace_settings.name = 'sRGB'
out_img.pixels = atlas.ravel()
out_img.filepath_raw = tex_path
out_img.file_format = 'PNG'
out_img.save()
# The tiles were rendered through the Standard view transform, so their
# pixels are already sRGB-encoded; the image is written as-is.


# ---- Stage 2: the model ----
scene = reset()


def box(bm_, x0, x1, y0, y1, z0, z1):
    geom = bmesh.ops.create_cube(bm_, size=1.0)
    for v in geom["verts"]:
        v.co.x = (x0 if v.co.x < 0 else x1) * IN
        v.co.y = (y0 if v.co.y < 0 else y1) * IN
        v.co.z = (z0 if v.co.z < 0 else z1) * IN
    return geom


def beveled_box(bm_, x0, x1, y0, y1, z0, z1, width_in):
    geom = box(bm_, x0, x1, y0, y1, z0, z1)
    verts = set(geom["verts"])
    edges = [e for e in bm_.edges if all(v in verts for v in e.verts)]
    bmesh.ops.bevel(bm_, geom=edges, offset=width_in * IN, segments=2,
                    affect='EDGES', profile=0.5)


def bisect(bm_, faces, co, no):
    verts = {v for f in faces for v in f.verts}
    edges = {e for f in faces for e in f.edges}
    bmesh.ops.bisect_plane(bm_, geom=list(verts) + list(edges) + list(faces),
                           plane_co=co, plane_no=no)


bm = bmesh.new()
# Head: hangs below the blade plane (z < 0) so its inner face (x = 0)
# hooks the board's edge while the blade lies flat on the board (z = 0).
beveled_box(bm, -HEAD_WIDTH, 0.0, HEAD_Y0, HEAD_Y1, -HEAD_THICK, 0.0, 0.02)
for row in range(1, HEAD_ROWS):
    yb = (HEAD_Y0 + row * head_seg) * IN
    bisect(bm, list(bm.faces), (0, yb, 0), (0, 1, 0))
head_faces = set(bm.faces)
# Blade: overlaps the head's top for riveting, then reaches 48" past it.
beveled_box(bm, X0, BLADE_REACH, -half_w, half_w, 0.0, BLADE_THICK, 0.01)
for row in range(1, BLADE_ROWS):
    xb = (X0 + row * blade_seg) * IN
    bisect(bm, [f for f in bm.faces if f not in head_faces], (xb, 0, 0), (1, 0, 0))

# Four rivets join blade and head.
for x in (-1.15, -0.35):
    for y in (-0.68, 0.68):
        geom = bmesh.ops.create_cone(bm, cap_ends=True, segments=24,
                                     radius1=0.12 * IN, radius2=0.10 * IN, depth=0.03 * IN)
        bmesh.ops.translate(bm, verts=geom["verts"],
                            vec=(x * IN, y * IN, (BLADE_THICK + 0.015) * IN))

# UVs: each top face samples its strip's band; every other face samples
# a plain aluminium texel in band 0's margin.
uv = bm.loops.layers.uv.new("UVMap")
plain = (0.5, 4.0 / TEX)
blade_top = BLADE_THICK * IN
for face in bm.faces:
    c = face.calc_center_median()
    up = face.normal.z > 0.999
    if up and abs(c.z - blade_top) < 1e-6:
        row = min(BLADE_ROWS - 1, max(0, int((c.x / IN - X0) // blade_seg)))
        strip = blade_strips[row]
    elif up and abs(c.z) < 1e-6:
        row = min(HEAD_ROWS - 1, max(0, int((c.y / IN - HEAD_Y0) // head_seg)))
        strip = head_strips[row]
    else:
        for loop in face.loops:
            loop[uv].uv = plain
        continue
    for loop in face.loops:
        loop[uv].uv = strip.uv(loop.vert.co.x / IN, loop.vert.co.y / IN)

mesh = bpy.data.meshes.new("DrywallSquare")
bm.to_mesh(mesh)
bm.free()
square = bpy.data.objects.new("DrywallSquare", mesh)
scene.collection.objects.link(square)

# The material name is what Unity's material override is keyed to.
mat = bpy.data.materials.new("Aluminium")
try:
    mat.use_nodes = True
except Exception:
    pass
nodes = mat.node_tree.nodes
bsdf = nodes.get("Principled BSDF")
bsdf.inputs["Metallic"].default_value = 1.0
bsdf.inputs["Roughness"].default_value = 0.32
tex_node = nodes.new("ShaderNodeTexImage")
tex_node.image = bpy.data.images.load(tex_path)
tex_node.image.colorspace_settings.name = 'sRGB'
mat.node_tree.links.new(tex_node.outputs["Color"], bsdf.inputs["Base Color"])
mat.diffuse_color = (*ALU, 1.0)
mesh.materials.append(mat)

bpy.context.view_layer.objects.active = square
square.select_set(True)
bpy.ops.object.shade_flat()

bpy.ops.wm.save_as_mainfile(filepath=os.path.join(blend_dir, "drywall-square.blend"),
                            relative_remap=True)
bpy.ops.export_scene.fbx(
    filepath=os.path.join(fbx_dir, "drywall-square.fbx"),
    use_selection=True,
    apply_unit_scale=True,
    apply_scale_options='FBX_SCALE_ALL',
    axis_forward='-Z',
    axis_up='Y',
    bake_space_transform=True,
    mesh_smooth_type='FACE',
    add_leaf_bones=False,
    path_mode='RELATIVE',
    embed_textures=False,
)

# ---- Previews (after saving, so not part of the .blend) ----
world = bpy.data.worlds.new("World")
scene.world = world
world.use_nodes = True
world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.55, 0.57, 0.6, 1)
world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.8

bpy.ops.mesh.primitive_plane_add(size=4.0, location=(0.55, 0, -HEAD_THICK * IN))
floor = bpy.context.active_object
floor.data.materials.append(material("Floor", (0.35, 0.33, 0.3), 0.0, 0.8))

sun = bpy.data.objects.new("Sun", bpy.data.lights.new("Sun", 'SUN'))
sun.data.energy = 3.0
sun.rotation_euler = (math.radians(40), math.radians(-20), math.radians(30))
scene.collection.objects.link(sun)

cam = bpy.data.objects.new("Cam", bpy.data.cameras.new("Cam"))
scene.collection.objects.link(cam)
scene.camera = cam
scene.render.engine = 'BLENDER_EEVEE'
scene.render.resolution_x = 1600
scene.render.resolution_y = 900

views = {
    "overview": ((0.6, -1.0, 1.1), (0.55, -0.05, 0.0), 35),
    "head-closeup": ((0.02, -0.06, 0.22), (0.0, -0.06, 0.0), 35),
    "stud-closeup": ((0.40, -0.07, 0.16), (0.40, 0.0, 0.0), 50),
}
for name, (loc, target, lens) in views.items():
    cam.location = loc
    cam.data.lens = lens
    look_at(cam, target)
    scene.render.filepath = os.path.join(preview_dir, f"{name}.png")
    bpy.ops.render.render(write_still=True)

print("PX_PER_IN", round(PPI, 1), "BANDS_PX", v_cursor)
print("DIMENSIONS_M", tuple(round(d, 4) for d in square.dimensions))
print("FACES", len(mesh.polygons), "MATERIALS", [m.name for m in mesh.materials])
