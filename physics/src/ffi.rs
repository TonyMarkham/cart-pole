//! The MuJoCo 3.15.0 C API this crate uses, declared by hand. Every
//! signature is copied from the 3.15.0 headers (`include\mujoco\`; line
//! numbers in the comments) and is valid only for that version —
//! `MJ_VERSION` is checked at load. State goes through `mj_getState` /
//! `mj_setState`. Site positions are not state (MuJoCo derives them from
//! `qpos`), so they are read from `mjData` itself, as is the pole's
//! centre of mass (`subtree_com`), and the actuator's `ctrlrange` and the
//! bodies' masses from `mjModel`: `mjModel` and `mjData` mirror only the
//! leading fields of their C structs, up to the last one read
//! (`actuator_ctrlrange`, `subtree_com`), and `load` proves the mirror
//! against the DLL before anything reads through it. `mjVFS` stays opaque.

// The names mirror the C header.
#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

use std::ffi::{c_char, c_int, c_void};

/// `mjVERSION_HEADER` (mujoco.h:19); `mj_version()` must return it.
pub const MJ_VERSION: c_int = 3015000;

// mjtObj (mjtype.h), counted from mjOBJ_UNKNOWN = 0.
/// mjtype.h:301
pub const mjOBJ_JOINT: c_int = 3;
/// mjtype.h:304
pub const mjOBJ_SITE: c_int = 6;
/// mjtype.h:317
pub const mjOBJ_ACTUATOR: c_int = 19;

// Array lengths inside mjModel's options.
/// mjmodel.h:40
pub const mjNREF: usize = 2;
/// mjmodel.h:41
pub const mjNIMP: usize = 5;

// Array lengths inside mjData's header.
/// mjmodel.h:44
pub const mjNSOLVER: usize = 200;
/// mjmodel.h:45
pub const mjNISLAND: usize = 20;
/// mjtype.h:566: the seven mjtWarning values above it.
pub const mjNWARNING: usize = 7;
/// mjtype.h:594: the fifteen mjtTimer values above it.
pub const mjNTIMER: usize = 15;

// mjtState bits (mjtype.h).
/// mjtype.h:509
pub const mjSTATE_TIME: c_int = 1 << 0;
/// mjtype.h:510
pub const mjSTATE_QPOS: c_int = 1 << 1;
/// mjtype.h:511
pub const mjSTATE_QVEL: c_int = 1 << 2;
/// mjtype.h:515
pub const mjSTATE_CTRL: c_int = 1 << 6;

// mjtLogLevel (mjtype.h), counted from mjLOG_DEBUG = 0.
/// mjtype.h:610
pub const mjLOG_WARNING: c_int = 2;
/// mjtype.h:611
pub const mjLOG_ERROR: c_int = 3;

// The leading fields of mjModel_ (mjmodel.h:240), mjtSize = int64_t
// (mjtype.h:52), mjtNum = double (mjtype.h:26), mjtByte and mjtBool = one
// byte (mjtype.h:41, 44), through actuator_ctrlrange (mjmodel.h:808), in
// header order; the section comments carry the header's own headings and
// lines. The C struct goes on past it; the type is only ever behind a
// pointer MuJoCo made, never built or copied in Rust. Fields before the
// ones read exist for their offsets; load proves the mirror (nq and nu
// against mj_stateSize, opt.timestep against the measured timestep, qpos0
// against the reset qpos, every pointer against MuJoCo's buffer, and
// actuator_ctrlrange against MuJoCo's own clamp).
#[repr(C)]
#[allow(dead_code)]
pub struct mjModel {
    // sizes (mjmodel.h:241)
    // sizes needed at mjModel construction (mjmodel.h:243)
    pub nq: i64,
    pub nv: i64,
    pub nu: i64,
    pub nactuator: i64,
    pub nout: i64,
    pub na: i64,
    pub nbody: i64,
    pub nbvh: i64,
    pub nbvhstatic: i64,
    pub nbvhdynamic: i64,
    pub noct: i64,
    pub njnt: i64,
    pub ntree: i64,
    pub nM: i64,
    pub nB: i64,
    pub nC: i64,
    pub nD: i64,
    pub ngeom: i64,
    pub nsite: i64,
    pub ncam: i64,
    pub nlight: i64,
    pub nflex: i64,
    pub nflexnode: i64,
    pub nflexvert: i64,
    pub nflexedge: i64,
    pub nflexelem: i64,
    pub nflexelemdata: i64,
    pub nflexstiffness: i64,
    pub nflexbending: i64,
    pub nefm0dof: i64,
    pub nefm0L: i64,
    pub nflexelemedge: i64,
    pub nflexshelldata: i64,
    pub nflextexcoord: i64,
    pub nJfe: i64,
    pub nJfv: i64,
    pub nmesh: i64,
    pub nmeshvert: i64,
    pub nmeshnormal: i64,
    pub nmeshtexcoord: i64,
    pub nmeshface: i64,
    pub nmeshgraph: i64,
    pub nmeshpoly: i64,
    pub nmeshpolyvert: i64,
    pub nmeshpolymap: i64,
    pub nskin: i64,
    pub nskinvert: i64,
    pub nskintexvert: i64,
    pub nskinface: i64,
    pub nskinbone: i64,
    pub nskinbonevert: i64,
    pub nhfield: i64,
    pub nhfielddata: i64,
    pub ntex: i64,
    pub ntexdata: i64,
    pub nmat: i64,
    pub npair: i64,
    pub nexclude: i64,
    pub neq: i64,
    pub ntendon: i64,
    pub nJten: i64,
    pub nwrap: i64,
    pub nsensor: i64,
    pub nnumeric: i64,
    pub nnumericdata: i64,
    pub ntext: i64,
    pub ntextdata: i64,
    pub ntuple: i64,
    pub ntupledata: i64,
    pub nkey: i64,
    pub nmocap: i64,
    pub nplugin: i64,
    pub npluginattr: i64,
    pub nuser_body: i64,
    pub nuser_jnt: i64,
    pub nuser_geom: i64,
    pub nuser_site: i64,
    pub nuser_cam: i64,
    pub nuser_tendon: i64,
    pub nuser_actuator: i64,
    pub nuser_sensor: i64,
    pub nnames: i64,
    pub npaths: i64,
    // sizes set after mjModel construction (mjmodel.h:328)
    pub nnames_map: i64,
    pub nJmom: i64,
    pub ngravcomp: i64,
    pub nemax: i64,
    pub njmax: i64,
    pub nconmax: i64,
    pub npolygonmax: i64,
    pub nmeshdegmax: i64,
    pub nuserdata: i64,
    pub nsensordata: i64,
    pub npluginstate: i64,
    pub nhistory: i64,
    // buffer sizes (mjmodel.h:342)
    pub narena: i64,
    pub nbuffer: i64,
    // flags (mjmodel.h:346)
    pub flg_gravcomp: u8,
    pub flg_surfacevel: u8,
    pub flg_adhesion: u8,
    // options and statistics (mjmodel.h:352)
    pub opt: mjOption,
    pub vis: mjVisual,
    pub stat: mjStatistic,
    // buffers (mjmodel.h:358)
    // main buffer (mjmodel.h:360)
    pub buffer: *mut c_void,
    // default generalized coordinates (mjmodel.h:363)
    pub qpos0: *mut f64,
    pub qpos_spring: *mut f64,
    // bodies (mjmodel.h:367)
    pub body_parentid: *mut c_int,
    pub body_rootid: *mut c_int,
    pub body_weldid: *mut c_int,
    pub body_mocapid: *mut c_int,
    pub body_jntnum: *mut c_int,
    pub body_jntadr: *mut c_int,
    pub body_dofnum: *mut c_int,
    pub body_dofadr: *mut c_int,
    pub body_treeid: *mut c_int,
    pub body_geomnum: *mut c_int,
    pub body_geomadr: *mut c_int,
    pub body_simple: *mut u8,
    pub body_sameframe: *mut u8,
    pub body_pos: *mut f64,
    pub body_quat: *mut f64,
    pub body_ipos: *mut f64,
    pub body_iquat: *mut f64,
    pub body_mass: *mut f64,
    pub body_subtreemass: *mut f64,
    pub body_inertia: *mut f64,
    pub body_invweight0: *mut f64,
    pub body_gravcomp: *mut f64,
    pub body_margin: *mut f64,
    pub body_user: *mut f64,
    pub body_plugin: *mut c_int,
    pub body_contype: *mut c_int,
    pub body_conaffinity: *mut c_int,
    pub body_bvhadr: *mut c_int,
    pub body_bvhnum: *mut c_int,
    // bounding volume hierarchy (mjmodel.h:398)
    pub bvh_depth: *mut c_int,
    pub bvh_child: *mut c_int,
    pub bvh_nodeid: *mut c_int,
    pub bvh_aabb: *mut f64,
    // octree spatial partitioning (mjmodel.h:404)
    pub oct_depth: *mut c_int,
    pub oct_child: *mut c_int,
    pub oct_aabb: *mut f64,
    pub oct_coeff: *mut f64,
    // joints (mjmodel.h:410)
    pub jnt_type: *mut c_int,
    pub jnt_qposadr: *mut c_int,
    pub jnt_dofadr: *mut c_int,
    pub jnt_bodyid: *mut c_int,
    pub jnt_actuatorid: *mut c_int,
    pub jnt_group: *mut c_int,
    pub jnt_limited: *mut u8,
    pub jnt_actfrclimited: *mut u8,
    pub jnt_actgravcomp: *mut u8,
    pub jnt_solref: *mut f64,
    pub jnt_solimp: *mut f64,
    pub jnt_pos: *mut f64,
    pub jnt_axis: *mut f64,
    pub jnt_stiffness: *mut f64,
    pub jnt_stiffnesspoly: *mut f64,
    pub jnt_range: *mut f64,
    pub jnt_actfrcrange: *mut f64,
    pub jnt_margin: *mut f64,
    pub jnt_user: *mut f64,
    // dofs (mjmodel.h:431)
    pub dof_bodyid: *mut c_int,
    pub dof_jntid: *mut c_int,
    pub dof_parentid: *mut c_int,
    pub dof_treeid: *mut c_int,
    pub dof_Madr: *mut c_int,
    pub dof_simplenum: *mut c_int,
    pub dof_solref: *mut f64,
    pub dof_solimp: *mut f64,
    pub dof_frictionloss: *mut f64,
    pub dof_armature: *mut f64,
    pub dof_damping: *mut f64,
    pub dof_dampingpoly: *mut f64,
    pub dof_invweight0: *mut f64,
    pub dof_M0: *mut f64,
    pub dof_length: *mut f64,
    // trees (mjmodel.h:448)
    pub tree_bodyadr: *mut c_int,
    pub tree_bodynum: *mut c_int,
    pub tree_dofadr: *mut c_int,
    pub tree_dofnum: *mut c_int,
    pub tree_sleep_policy: *mut c_int,
    // geoms (mjmodel.h:455)
    pub geom_type: *mut c_int,
    pub geom_contype: *mut c_int,
    pub geom_conaffinity: *mut c_int,
    pub geom_condim: *mut c_int,
    pub geom_bodyid: *mut c_int,
    pub geom_dataid: *mut c_int,
    pub geom_matid: *mut c_int,
    pub geom_group: *mut c_int,
    pub geom_priority: *mut c_int,
    pub geom_plugin: *mut c_int,
    pub geom_sameframe: *mut u8,
    pub geom_solmix: *mut f64,
    pub geom_solref: *mut f64,
    pub geom_solimp: *mut f64,
    pub geom_size: *mut f64,
    pub geom_aabb: *mut f64,
    pub geom_rbound: *mut f64,
    pub geom_pos: *mut f64,
    pub geom_quat: *mut f64,
    pub geom_friction: *mut f64,
    pub geom_margin: *mut f64,
    pub geom_gap: *mut f64,
    pub geom_surfacevel: *mut f64,
    pub geom_adhesion: *mut f64,
    pub geom_fluid: *mut f64,
    pub geom_user: *mut f64,
    pub geom_rgba: *mut f32,
    // sites (mjmodel.h:484)
    pub site_type: *mut c_int,
    pub site_bodyid: *mut c_int,
    pub site_dataid: *mut c_int,
    pub site_matid: *mut c_int,
    pub site_group: *mut c_int,
    pub site_sameframe: *mut u8,
    pub site_size: *mut f64,
    pub site_pos: *mut f64,
    pub site_quat: *mut f64,
    pub site_user: *mut f64,
    pub site_rgba: *mut f32,
    // cameras (mjmodel.h:497)
    pub cam_mode: *mut c_int,
    pub cam_bodyid: *mut c_int,
    pub cam_targetbodyid: *mut c_int,
    pub cam_pos: *mut f64,
    pub cam_quat: *mut f64,
    pub cam_poscom0: *mut f64,
    pub cam_pos0: *mut f64,
    pub cam_mat0: *mut f64,
    pub cam_projection: *mut c_int,
    pub cam_fovy: *mut f64,
    pub cam_ipd: *mut f64,
    pub cam_resolution: *mut c_int,
    pub cam_output: *mut c_int,
    pub cam_sensorsize: *mut f32,
    pub cam_intrinsic: *mut f32,
    pub cam_user: *mut f64,
    // lights (mjmodel.h:515)
    pub light_mode: *mut c_int,
    pub light_bodyid: *mut c_int,
    pub light_targetbodyid: *mut c_int,
    pub light_type: *mut c_int,
    pub light_texid: *mut c_int,
    pub light_castshadow: *mut u8,
    pub light_bulbradius: *mut f32,
    pub light_intensity: *mut f32,
    pub light_range: *mut f32,
    pub light_active: *mut u8,
    pub light_pos: *mut f64,
    pub light_dir: *mut f64,
    pub light_poscom0: *mut f64,
    pub light_pos0: *mut f64,
    pub light_dir0: *mut f64,
    pub light_attenuation: *mut f32,
    pub light_cutoff: *mut f32,
    pub light_softness: *mut f32,
    pub light_exponent: *mut f32,
    pub light_ambient: *mut f32,
    pub light_diffuse: *mut f32,
    pub light_specular: *mut f32,
    // flexes: contact properties (mjmodel.h:539)
    pub flex_contype: *mut c_int,
    pub flex_conaffinity: *mut c_int,
    pub flex_condim: *mut c_int,
    pub flex_priority: *mut c_int,
    pub flex_solmix: *mut f64,
    pub flex_solref: *mut f64,
    pub flex_solimp: *mut f64,
    pub flex_friction: *mut f64,
    pub flex_margin: *mut f64,
    pub flex_gap: *mut f64,
    pub flex_selfcollide: *mut c_int,
    pub flex_activelayers: *mut c_int,
    pub flex_passive: *mut c_int,
    // flexes: other properties (mjmodel.h:554)
    pub flex_dim: *mut c_int,
    pub flex_matid: *mut c_int,
    pub flex_group: *mut c_int,
    pub flex_interp: *mut c_int,
    pub flex_cellnum: *mut c_int,
    pub flex_nodeadr: *mut c_int,
    pub flex_nodenum: *mut c_int,
    pub flex_vertadr: *mut c_int,
    pub flex_vertnum: *mut c_int,
    pub flex_edgeadr: *mut c_int,
    pub flex_edgenum: *mut c_int,
    pub flex_elemadr: *mut c_int,
    pub flex_elemnum: *mut c_int,
    pub flex_elemdataadr: *mut c_int,
    pub flex_stiffnessadr: *mut c_int,
    pub flex_elemedgeadr: *mut c_int,
    pub flex_bendingadr: *mut c_int,
    pub flex_shellnum: *mut c_int,
    pub flex_shelldataadr: *mut c_int,
    pub flex_texcoordadr: *mut c_int,
    pub flex_nodebodyid: *mut c_int,
    pub flex_vertbodyid: *mut c_int,
    pub flex_vertedgeadr: *mut c_int,
    pub flex_vertedgenum: *mut c_int,
    pub flex_vertedge: *mut c_int,
    pub flex_edge: *mut c_int,
    pub flex_edgeflap: *mut c_int,
    pub flex_elem: *mut c_int,
    pub flex_elemtexcoord: *mut c_int,
    pub flex_elemedge: *mut c_int,
    pub flex_elemlayer: *mut c_int,
    pub flex_shell: *mut c_int,
    pub flex_vert: *mut f64,
    pub flex_vert0: *mut f64,
    pub flex_vertmetric: *mut f64,
    pub flex_node: *mut f64,
    pub flex_node0: *mut f64,
    pub flexedge_length0: *mut f64,
    pub flexedge_invweight0: *mut f64,
    pub flex_radius: *mut f64,
    pub flex_size: *mut f64,
    pub flex_stiffness: *mut f64,
    pub flex_bending: *mut f64,
    pub efm0_dofid: *mut c_int,
    pub efm0_L_rownnz: *mut c_int,
    pub efm0_L_rowadr: *mut c_int,
    pub efm0_L_colind: *mut c_int,
    pub efm0_L: *mut f64,
    pub flex_damping: *mut f64,
    pub flex_edgestiffness: *mut f64,
    pub flex_edgedamping: *mut f64,
    pub flex_edgeequality: *mut c_int,
    pub flex_rigid: *mut u8,
    pub flexedge_rigid: *mut u8,
    pub flex_centered: *mut u8,
    pub flex_flatskin: *mut u8,
    pub flex_bvhadr: *mut c_int,
    pub flex_bvhnum: *mut c_int,
    pub flexedge_J_rownnz: *mut c_int,
    pub flexedge_J_rowadr: *mut c_int,
    pub flexedge_J_colind: *mut c_int,
    pub flexvert_J_rownnz: *mut c_int,
    pub flexvert_J_rowadr: *mut c_int,
    pub flexvert_J_colind: *mut c_int,
    pub flex_rgba: *mut f32,
    pub flex_texcoord: *mut f32,
    // meshes (mjmodel.h:622)
    pub mesh_vertadr: *mut c_int,
    pub mesh_vertnum: *mut c_int,
    pub mesh_faceadr: *mut c_int,
    pub mesh_facenum: *mut c_int,
    pub mesh_bvhadr: *mut c_int,
    pub mesh_bvhnum: *mut c_int,
    pub mesh_octadr: *mut c_int,
    pub mesh_octnum: *mut c_int,
    pub mesh_normaladr: *mut c_int,
    pub mesh_normalnum: *mut c_int,
    pub mesh_texcoordadr: *mut c_int,
    pub mesh_texcoordnum: *mut c_int,
    pub mesh_graphadr: *mut c_int,
    pub mesh_extrema: *mut c_int,
    pub mesh_vert: *mut f32,
    pub mesh_normal: *mut f32,
    pub mesh_texcoord: *mut f32,
    pub mesh_face: *mut c_int,
    pub mesh_facenormal: *mut c_int,
    pub mesh_facetexcoord: *mut c_int,
    pub mesh_graph: *mut c_int,
    pub mesh_scale: *mut f64,
    pub mesh_pos: *mut f64,
    pub mesh_quat: *mut f64,
    pub mesh_pathadr: *mut c_int,
    pub mesh_polynum: *mut c_int,
    pub mesh_polyadr: *mut c_int,
    pub mesh_polynormal: *mut f64,
    pub mesh_polyvertadr: *mut c_int,
    pub mesh_polyvertnum: *mut c_int,
    pub mesh_polyvert: *mut c_int,
    pub mesh_polymapadr: *mut c_int,
    pub mesh_polymapnum: *mut c_int,
    pub mesh_polymap: *mut c_int,
    // skins (mjmodel.h:658)
    pub skin_matid: *mut c_int,
    pub skin_group: *mut c_int,
    pub skin_rgba: *mut f32,
    pub skin_inflate: *mut f32,
    pub skin_vertadr: *mut c_int,
    pub skin_vertnum: *mut c_int,
    pub skin_texcoordadr: *mut c_int,
    pub skin_faceadr: *mut c_int,
    pub skin_facenum: *mut c_int,
    pub skin_boneadr: *mut c_int,
    pub skin_bonenum: *mut c_int,
    pub skin_vert: *mut f32,
    pub skin_texcoord: *mut f32,
    pub skin_face: *mut c_int,
    pub skin_bonevertadr: *mut c_int,
    pub skin_bonevertnum: *mut c_int,
    pub skin_bonebindpos: *mut f32,
    pub skin_bonebindquat: *mut f32,
    pub skin_bonebodyid: *mut c_int,
    pub skin_bonevertid: *mut c_int,
    pub skin_bonevertweight: *mut f32,
    pub skin_pathadr: *mut c_int,
    // height fields (mjmodel.h:682)
    pub hfield_size: *mut f64,
    pub hfield_nrow: *mut c_int,
    pub hfield_ncol: *mut c_int,
    pub hfield_adr: *mut c_int,
    pub hfield_data: *mut f32,
    pub hfield_pathadr: *mut c_int,
    // textures (mjmodel.h:690)
    pub tex_type: *mut c_int,
    pub tex_colorspace: *mut c_int,
    pub tex_height: *mut c_int,
    pub tex_width: *mut c_int,
    pub tex_nchannel: *mut c_int,
    pub tex_adr: *mut i64,
    pub tex_data: *mut u8,
    pub tex_pathadr: *mut c_int,
    // materials (mjmodel.h:700)
    pub mat_texid: *mut c_int,
    pub mat_texuniform: *mut u8,
    pub mat_texrepeat: *mut f32,
    pub mat_emission: *mut f32,
    pub mat_specular: *mut f32,
    pub mat_shininess: *mut f32,
    pub mat_reflectance: *mut f32,
    pub mat_metallic: *mut f32,
    pub mat_roughness: *mut f32,
    pub mat_rgba: *mut f32,
    // predefined geom pairs for collision detection; has precedence over exclude (mjmodel.h:712)
    pub pair_dim: *mut c_int,
    pub pair_geom1: *mut c_int,
    pub pair_geom2: *mut c_int,
    pub pair_signature: *mut c_int,
    pub pair_solref: *mut f64,
    pub pair_solreffriction: *mut f64,
    pub pair_solimp: *mut f64,
    pub pair_margin: *mut f64,
    pub pair_gap: *mut f64,
    pub pair_adhesion: *mut f64,
    pub pair_friction: *mut f64,
    // excluded body pairs for collision detection (mjmodel.h:725)
    pub exclude_signature: *mut c_int,
    // equality constraints (mjmodel.h:728)
    pub eq_type: *mut c_int,
    pub eq_obj1id: *mut c_int,
    pub eq_obj2id: *mut c_int,
    pub eq_objtype: *mut c_int,
    pub eq_active0: *mut u8,
    pub eq_solref: *mut f64,
    pub eq_solimp: *mut f64,
    pub eq_data: *mut f64,
    // tendons (mjmodel.h:738)
    pub tendon_adr: *mut c_int,
    pub tendon_num: *mut c_int,
    pub tendon_matid: *mut c_int,
    pub tendon_actuatorid: *mut c_int,
    pub tendon_group: *mut c_int,
    pub tendon_treenum: *mut c_int,
    pub tendon_treeid: *mut c_int,
    pub ten_J_rownnz: *mut c_int,
    pub ten_J_rowadr: *mut c_int,
    pub ten_J_colind: *mut c_int,
    pub tendon_limited: *mut u8,
    pub tendon_actfrclimited: *mut u8,
    pub tendon_width: *mut f64,
    pub tendon_solref_lim: *mut f64,
    pub tendon_solimp_lim: *mut f64,
    pub tendon_solref_fri: *mut f64,
    pub tendon_solimp_fri: *mut f64,
    pub tendon_range: *mut f64,
    pub tendon_actfrcrange: *mut f64,
    pub tendon_margin: *mut f64,
    pub tendon_stiffness: *mut f64,
    pub tendon_stiffnesspoly: *mut f64,
    pub tendon_damping: *mut f64,
    pub tendon_dampingpoly: *mut f64,
    pub tendon_armature: *mut f64,
    pub tendon_frictionloss: *mut f64,
    pub tendon_lengthspring: *mut f64,
    pub tendon_length0: *mut f64,
    pub tendon_invweight0: *mut f64,
    pub tendon_user: *mut f64,
    pub tendon_rgba: *mut f32,
    // list of all wrap objects in tendon paths (mjmodel.h:771)
    pub wrap_type: *mut c_int,
    pub wrap_objid: *mut c_int,
    pub wrap_prm: *mut f64,
    // actuators (mjmodel.h:776)
    pub actuator_trntype: *mut c_int,
    pub actuator_dyntype: *mut c_int,
    pub actuator_gaintype: *mut c_int,
    pub actuator_biastype: *mut c_int,
    pub actuator_ctrladr: *mut c_int,
    pub actuator_ctrlnum: *mut c_int,
    pub actuator_ctrlspec: *mut c_int,
    pub actuator_outadr: *mut c_int,
    pub actuator_outnum: *mut c_int,
    pub actuator_actadr: *mut c_int,
    pub actuator_actnum: *mut c_int,
    pub actuator_trnid: *mut c_int,
    pub actuator_cranklength: *mut f64,
    pub actuator_dynprm: *mut f64,
    pub actuator_gainprm: *mut f64,
    pub actuator_biasprm: *mut f64,
    pub actuator_actlimited: *mut u8,
    pub actuator_actrange: *mut f64,
    pub actuator_actearly: *mut u8,
    pub actuator_history: *mut c_int,
    pub actuator_historyadr: *mut c_int,
    pub actuator_delay: *mut f64,
    pub actuator_damping: *mut f64,
    pub actuator_dampingpoly: *mut f64,
    pub actuator_armature: *mut f64,
    pub actuator_group: *mut c_int,
    pub actuator_user: *mut f64,
    pub actuator_plugin: *mut c_int,
    pub actuator_forcelimited: *mut u8,
    pub actuator_forcerange: *mut f64,
    pub actuator_ctrllimited: *mut u8,
    pub actuator_ctrlrange: *mut f64,
}

/// mjmodel.h:81
#[repr(C)]
#[allow(dead_code)]
pub struct mjOption {
    // timing parameters (mjmodel.h:82)
    pub timestep: f64,
    // solver parameters (mjmodel.h:85)
    pub impratio: f64,
    pub tolerance: f64,
    pub ls_tolerance: f64,
    pub noslip_tolerance: f64,
    pub ccd_tolerance: f64,
    // sleep settings (mjmodel.h:92)
    pub sleep_tolerance: f64,
    // physical constants (mjmodel.h:95)
    pub gravity: [f64; 3],
    pub wind: [f64; 3],
    pub magnetic: [f64; 3],
    pub density: f64,
    pub viscosity: f64,
    // override contact solver parameters (if enabled) (mjmodel.h:102)
    pub o_margin: f64,
    pub o_solref: [f64; mjNREF],
    pub o_solimp: [f64; mjNIMP],
    pub o_friction: [f64; 5],
    // discrete settings (mjmodel.h:108)
    pub integrator: c_int,
    pub cone: c_int,
    pub jacobian: c_int,
    pub solver: c_int,
    pub iterations: c_int,
    pub ls_iterations: c_int,
    pub noslip_iterations: c_int,
    pub ccd_iterations: c_int,
    pub disableflags: c_int,
    pub enableflags: c_int,
    pub disableactuator: c_int,
    // sdf collision settings (mjmodel.h:121)
    pub sdf_initpoints: c_int,
    pub sdf_iterations: c_int,
}

/// mjmodel.h:129: six nested structs of 4-byte fields.
#[repr(C)]
#[allow(dead_code)]
pub struct mjVisual {
    pub global: mjVisualGlobal,
    pub quality: mjVisualQuality,
    pub headlight: mjVisualHeadlight,
    pub map: mjVisualMap,
    pub scale: mjVisualScale,
    pub rgba: mjVisualRgba,
}

/// mjVisual's `global` struct (mjmodel.h:130).
#[repr(C)]
#[allow(dead_code)]
pub struct mjVisualGlobal {
    pub cameraid: c_int,
    pub orthographic: c_int,
    pub fovy: f32,
    pub ipd: f32,
    pub azimuth: f32,
    pub elevation: f32,
    pub linewidth: f32,
    pub glow: f32,
    pub realtime: f32,
    pub offwidth: c_int,
    pub offheight: c_int,
    pub ellipsoidinertia: c_int,
    pub bvactive: c_int,
}

/// mjVisual's `quality` struct (mjmodel.h:146).
#[repr(C)]
#[allow(dead_code)]
pub struct mjVisualQuality {
    pub shadowsize: c_int,
    pub offsamples: c_int,
    pub numslices: c_int,
    pub numstacks: c_int,
    pub numquads: c_int,
}

/// mjVisual's `headlight` struct (mjmodel.h:154).
#[repr(C)]
#[allow(dead_code)]
pub struct mjVisualHeadlight {
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub active: c_int,
}

/// mjVisual's `map` struct (mjmodel.h:161).
#[repr(C)]
#[allow(dead_code)]
pub struct mjVisualMap {
    pub stiffness: f32,
    pub stiffnessrot: f32,
    pub force: f32,
    pub torque: f32,
    pub alpha: f32,
    pub fogstart: f32,
    pub fogend: f32,
    pub znear: f32,
    pub zfar: f32,
    pub haze: f32,
    pub shadowclip: f32,
    pub shadowscale: f32,
    pub actuatortendon: f32,
}

/// mjVisual's `scale` struct (mjmodel.h:177).
#[repr(C)]
#[allow(dead_code)]
pub struct mjVisualScale {
    pub forcewidth: f32,
    pub contactwidth: f32,
    pub contactheight: f32,
    pub connect: f32,
    pub com: f32,
    pub camera: f32,
    pub light: f32,
    pub selectpoint: f32,
    pub jointlength: f32,
    pub jointwidth: f32,
    pub actuatorlength: f32,
    pub actuatorwidth: f32,
    pub framelength: f32,
    pub framewidth: f32,
    pub constraint: f32,
    pub slidercrank: f32,
    pub frustum: f32,
}

/// mjVisual's `rgba` struct (mjmodel.h:197).
#[repr(C)]
#[allow(dead_code)]
pub struct mjVisualRgba {
    pub fog: [f32; 4],
    pub haze: [f32; 4],
    pub force: [f32; 4],
    pub inertia: [f32; 4],
    pub joint: [f32; 4],
    pub actuator: [f32; 4],
    pub actuatornegative: [f32; 4],
    pub actuatorpositive: [f32; 4],
    pub com: [f32; 4],
    pub camera: [f32; 4],
    pub light: [f32; 4],
    pub selectpoint: [f32; 4],
    pub connect: [f32; 4],
    pub contactpoint: [f32; 4],
    pub contactforce: [f32; 4],
    pub contactfriction: [f32; 4],
    pub contacttorque: [f32; 4],
    pub contactgap: [f32; 4],
    pub rangefinder: [f32; 4],
    pub constraint: [f32; 4],
    pub slidercrank: [f32; 4],
    pub crankbroken: [f32; 4],
    pub frustum: [f32; 4],
    pub bv: [f32; 4],
    pub bvactive: [f32; 4],
}

/// mjmodel.h:229
#[repr(C)]
#[allow(dead_code)]
pub struct mjStatistic {
    pub meaninertia: f64,
    pub meanmass: f64,
    pub meansize: f64,
    pub extent: f64,
    pub center: [f64; 3],
}

/// mjdata.h:86
#[repr(C)]
#[allow(dead_code)]
pub struct mjSolverStat {
    pub improvement: f64,
    pub gradient: f64,
    pub lineslope: f64,
    pub nactive: c_int,
    pub nchange: c_int,
    pub neval: c_int,
    pub nupdate: c_int,
}

/// mjdata.h:74
#[repr(C)]
#[allow(dead_code)]
pub struct mjWarningStat {
    pub lastinfo: c_int,
    pub number: c_int,
}

/// mjdata.h:80
#[repr(C)]
#[allow(dead_code)]
pub struct mjTimerStat {
    pub duration: f64,
    pub number: c_int,
}

// The leading fields of mjData_ (mjdata.h:99) through subtree_com
// (mjdata.h:232), in header order. mjtNum = double (mjtype.h:26), mjtBool
// = one byte (mjtype.h:44), size_t and uintptr_t = usize. The C struct
// goes on past subtree_com; like mjModel it is only ever behind a pointer
// MuJoCo made. Fields before the ones read exist for their offsets; load
// checks time, qpos and site_xpos against mj_getState and the buffer
// bounds, and subtree_com against the centres it must hold (lib.rs,
// check_pole).
#[repr(C)]
#[allow(dead_code)]
pub struct mjData {
    pub narena: i64,
    pub nbuffer: i64,
    pub nplugin: c_int,
    pub pstack: usize,
    pub pbase: usize,
    pub parena: usize,
    pub threadpool: usize,
    pub threadlock: u8,
    pub maxuse_stack: i64,
    pub maxuse_arena: i64,
    pub maxuse_con: c_int,
    pub maxuse_efc: c_int,
    pub solver: [mjSolverStat; mjNISLAND * mjNSOLVER],
    pub solver_niter: [c_int; mjNISLAND],
    pub solver_nnz: [c_int; mjNISLAND],
    pub solver_fwdinv: [f64; 2],
    pub warning: [mjWarningStat; mjNWARNING],
    pub timer: [mjTimerStat; mjNTIMER],
    pub ncon: c_int,
    pub ne: c_int,
    pub nf: c_int,
    pub nl: c_int,
    pub nefc: c_int,
    pub nJ: c_int,
    pub efm_active: c_int,
    pub nefmK: c_int,
    pub nefmcon: c_int,
    pub nefmT: c_int,
    pub nefmA: c_int,
    pub nefmdof: c_int,
    pub nefmL: c_int,
    pub nY: c_int,
    pub nA: c_int,
    pub nisland: c_int,
    pub nidof: c_int,
    pub ntree_awake: c_int,
    pub nbody_awake: c_int,
    pub nparent_awake: c_int,
    pub nv_awake: c_int,
    pub flg_energypos: u8,
    pub flg_energyvel: u8,
    pub flg_subtreevel: u8,
    pub flg_rnepost: u8,
    pub time: f64,
    pub energy: [f64; 2],
    pub buffer: *mut c_void,
    pub arena: *mut c_void,
    pub qpos: *mut f64,
    pub qvel: *mut f64,
    pub act: *mut f64,
    pub history: *mut f64,
    pub qacc_warmstart: *mut f64,
    pub plugin_state: *mut f64,
    pub ctrl: *mut f64,
    pub qfrc_applied: *mut f64,
    pub xfrc_applied: *mut f64,
    pub eq_active: *mut u8,
    pub mocap_pos: *mut f64,
    pub mocap_quat: *mut f64,
    pub qacc: *mut f64,
    pub act_dot: *mut f64,
    pub userdata: *mut f64,
    pub sensordata: *mut f64,
    pub tree_asleep: *mut c_int,
    pub plugin: *mut c_int,
    pub plugin_data: *mut usize,
    pub xpos: *mut f64,
    pub xquat: *mut f64,
    pub xmat: *mut f64,
    pub xipos: *mut f64,
    pub ximat: *mut f64,
    pub xanchor: *mut f64,
    pub xaxis: *mut f64,
    pub geom_xpos: *mut f64,
    pub geom_xmat: *mut f64,
    pub site_xpos: *mut f64,
    pub site_xmat: *mut f64,
    pub cam_xpos: *mut f64,
    pub cam_xmat: *mut f64,
    pub light_xpos: *mut f64,
    pub light_xdir: *mut f64,
    // computed by mj_fwdPosition/mj_comPos (mjdata.h:231)
    pub subtree_com: *mut f64,
}

#[repr(C)]
pub struct mjVFS {
    _opaque: [u8; 0],
}

/// mjtype.h:625. `timestamp` is `mjtBool` (C `_Bool`, one byte); it is
/// read as `u8` because a Rust `bool` holding anything but 0 or 1 is
/// undefined behavior.
#[repr(C)]
pub struct mjLogMessage {
    pub level: c_int,
    pub topic: c_int,
    pub subject: [c_char; 1024],
    pub body: *const c_char,
    pub func: *const c_char,
    pub file: *const c_char,
    pub line: c_int,
    pub timestamp: u8,
}

/// mjtype.h:644; `None` is C's NULL (restores MuJoCo's default handler).
pub type mjfLogHandler = Option<unsafe extern "C" fn(*const mjLogMessage)>;

unsafe extern "C" {
    /// mujoco.h:132
    pub fn mj_loadXML(
        filename: *const c_char,
        vfs: *const mjVFS,
        error: *mut c_char,
        error_sz: c_int,
    ) -> *mut mjModel;
    /// mujoco.h:189
    pub fn mj_step(m: *const mjModel, d: *mut mjData);
    /// mujoco.h:198
    pub fn mj_forward(m: *const mjModel, d: *mut mjData);
    /// mujoco.h:427; computes body, geom and site poses from qpos.
    pub fn mj_kinematics(m: *const mjModel, d: *mut mjData);
    /// mujoco.h:430; computes subtree_com (and the com-based inertias)
    /// from mj_kinematics' poses.
    pub fn mj_comPos(m: *const mjModel, d: *mut mjData);
    /// mujoco.h:617; res = M vec, M the inertia matrix mj_forward made.
    pub fn mj_mulM(m: *const mjModel, d: *const mjData, res: *mut f64, vec: *const f64);
    /// mujoco.h:666; the sum of every body's mass.
    pub fn mj_getTotalmass(m: *const mjModel) -> f64;
    /// mujoco.h:242
    pub fn mj_deleteModel(m: *mut mjModel);
    /// mujoco.h:249
    pub fn mj_makeData(m: *const mjModel) -> *mut mjData;
    /// mujoco.h:262
    pub fn mj_resetData(m: *const mjModel, d: *mut mjData);
    /// mujoco.h:292
    pub fn mj_deleteData(d: *mut mjData);
    /// mujoco.h:505
    pub fn mj_stateSize(m: *const mjModel, sig: c_int) -> c_int;
    /// mujoco.h:508
    pub fn mj_getState(m: *const mjModel, d: *const mjData, state: *mut f64, sig: c_int);
    /// mujoco.h:515
    pub fn mj_setState(m: *const mjModel, d: *mut mjData, state: *const f64, sig: c_int);
    /// mujoco.h:604; returns -1 when no object of that type has the name.
    pub fn mj_name2id(m: *const mjModel, type_: c_int, name: *const c_char) -> c_int;
    /// mujoco.h:646; 1 when `point` (world frame) is inside the site's
    /// volume, read from MuJoCo's own copy of the site pose.
    pub fn mj_insideSite(
        m: *const mjModel,
        d: *const mjData,
        siteid: c_int,
        point: *const f64,
    ) -> c_int;
    /// mujoco.h:684
    pub fn mj_version() -> c_int;
    /// mujoco.h:1007; returns the previous handler.
    pub fn mju_setLogHandler(handler: mjfLogHandler) -> mjfLogHandler;
}
