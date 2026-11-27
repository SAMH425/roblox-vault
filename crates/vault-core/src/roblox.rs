//! Pipeline de extracción para modelos de Roblox (.rbxm binario y .rbxmx XML)
//! usando rbx_binary / rbx_xml (los mismos lectores que usa Rojo).
//!
//! Resultado: jerarquía, piezas con tamaño/CFrame/color/material (para dibujarlas
//! en Three.js), scripts con su código y estadísticas.
//! Limitación: los MeshPart guardan solo un enlace a la malla en la nube de Roblox
//! (MeshId); sin descargarla se dibujan con su caja de tamaño real y se marcan como `mesh`.

use rbx_dom_weak::types::{Variant, Vector3};
use rbx_dom_weak::{Instance, WeakDom};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Serialize, Debug, Clone)]
pub struct RbxPart {
    pub path: String,
    pub class: String,
    /// "block" | "ball" | "cylinder" | "wedge" | "cornerwedge" | "mesh"
    pub shape: String,
    pub size: [f32; 3],
    pub position: [f32; 3],
    /// Matriz de rotación por filas (3x3) como en CFrame de Roblox.
    pub rotation: [f32; 9],
    pub color: [u8; 3],
    pub material: String,
    pub transparency: f32,
    pub mesh_id: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RbxNode { pub name: String, pub class: String, pub children: Vec<RbxNode> }

#[derive(Serialize, Debug, Clone)]
pub struct RbxScript { pub path: String, pub class: String, pub source: String }

#[derive(Serialize, Debug, Clone)]
pub struct RbxAnimation { pub path: String, pub keyframes: usize, pub length: f32, pub looped: bool, pub priority: Option<u32>, pub poses: Vec<String> }

#[derive(Serialize, Debug, Clone)]
pub struct RbxModel {
    pub tree: Vec<RbxNode>,
    pub parts: Vec<RbxPart>,
    pub scripts: Vec<RbxScript>,
    pub animations: Vec<RbxAnimation>,
    pub instance_count: usize,
    pub class_counts: BTreeMap<String, usize>,
    pub materials: BTreeMap<String, usize>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

pub fn parse(bytes: &[u8], format: &str) -> Result<RbxModel, String> {
    let dom = match format {
        "rbxm" | "rbxl" => rbx_binary::from_reader(bytes).map_err(|e| format!("RBXM no válido: {e}"))?,
        "rbxmx" | "rbxlx" => rbx_xml::from_reader_default(bytes).map_err(|e| format!("RBXMX no válido: {e}"))?,
        _ => {
            // detección por cabecera
            let mut head = [0u8; 8];
            (&bytes[..]).read(&mut head).ok();
            if &head == b"<roblox!" { rbx_binary::from_reader(bytes).map_err(|e| e.to_string())? }
            else { rbx_xml::from_reader_default(bytes).map_err(|e| e.to_string())? }
        }
    };
    Ok(extract(&dom))
}

fn material_name(v: u32) -> String {
    match v {
        256 => "Plastic", 272 => "SmoothPlastic", 288 => "Neon", 512 => "Wood", 528 => "WoodPlanks", 784 => "Marble",
        800 => "Slate", 816 => "Concrete", 832 => "Granite", 848 => "Brick", 864 => "Pebble", 880 => "Cobblestone",
        1040 => "CorrodedMetal", 1056 => "DiamondPlate", 1072 => "Foil", 1088 => "Metal", 1280 => "Grass", 1296 => "Sand",
        1312 => "Fabric", 1536 => "Ice", 1568 => "Glass", 1584 => "ForceField",
        _ => return format!("Material({v})"),
    }.into()
}

fn enum_val(i: &Instance, key: &str) -> Option<u32> {
    match i.properties.get(&key.into())? { Variant::Enum(e) => Some(e.to_u32()), Variant::EnumItem(e) => Some(e.value), _ => None }
}
fn vec3(i: &Instance, key: &str) -> Option<Vector3> {
    match i.properties.get(&key.into())? { Variant::Vector3(v) => Some(*v), _ => None }
}
fn f32p(i: &Instance, key: &str) -> Option<f32> {
    match i.properties.get(&key.into())? { Variant::Float32(f) => Some(*f), Variant::Float64(f) => Some(*f as f32), _ => None }
}
fn color(i: &Instance) -> [u8; 3] {
    for k in ["Color", "Color3uint8"] {
        match i.properties.get(&k.into()) {
            Some(Variant::Color3uint8(c)) => return [c.r, c.g, c.b],
            Some(Variant::Color3(c)) => return [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8],
            _ => {}
        }
    }
    [163, 162, 165] // gris medio por defecto de Roblox
}
fn text(i: &Instance, key: &str) -> Option<String> {
    match i.properties.get(&key.into())? {
        Variant::String(s) => Some(s.clone()),
        Variant::ContentId(c) => Some(c.as_str().to_string()),
        Variant::Content(c) => c.as_uri().map(|s| s.to_string()),
        Variant::BinaryString(b) => String::from_utf8(AsRef::<[u8]>::as_ref(b).to_vec()).ok(),
        _ => None,
    }
}

const PART_CLASSES: &[&str] = &["Part", "WedgePart", "CornerWedgePart", "MeshPart", "TrussPart", "SpawnLocation", "Seat", "VehicleSeat", "UnionOperation", "NegateOperation", "IntersectOperation"];
const SCRIPT_CLASSES: &[&str] = &["Script", "LocalScript", "ModuleScript"];

fn extract(dom: &WeakDom) -> RbxModel {
    let mut m = RbxModel { tree: vec![], parts: vec![], scripts: vec![], animations: vec![], instance_count: 0, class_counts: BTreeMap::new(), materials: BTreeMap::new(), bounds_min: [f32::MAX; 3], bounds_max: [f32::MIN; 3] };
    fn walk(dom: &WeakDom, i: &Instance, path: &str, m: &mut RbxModel) -> RbxNode {
        m.instance_count += 1;
        let class = i.class.as_str().to_string();
        *m.class_counts.entry(class.clone()).or_default() += 1;
        let path = if path.is_empty() { i.name.clone() } else { format!("{path}/{}", i.name) };
        if PART_CLASSES.contains(&class.as_str()) {
            let size = vec3(i, "size").or_else(|| vec3(i, "Size")).unwrap_or(Vector3::new(4.0, 1.0, 2.0));
            let cf = match i.properties.get(&"CFrame".into()) { Some(Variant::CFrame(c)) => Some(*c), _ => None };
            let (pos, rot) = cf.map(|c| ([c.position.x, c.position.y, c.position.z],
                [c.orientation.x.x, c.orientation.x.y, c.orientation.x.z, c.orientation.y.x, c.orientation.y.y, c.orientation.y.z, c.orientation.z.x, c.orientation.z.y, c.orientation.z.z]))
                .unwrap_or(([0.0; 3], [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]));
            let shape = match class.as_str() {
                "WedgePart" => "wedge".into(), "CornerWedgePart" => "cornerwedge".into(),
                "MeshPart" | "UnionOperation" | "NegateOperation" | "IntersectOperation" => "mesh".into(),
                _ => match enum_val(i, "shape").or_else(|| enum_val(i, "Shape")) { Some(0) => "ball", Some(2) => "cylinder", Some(3) => "wedge", Some(4) => "cornerwedge", _ => "block" }.into(),
            };
            let material = enum_val(i, "Material").map(material_name).unwrap_or_else(|| "Plastic".into());
            *m.materials.entry(material.clone()).or_default() += 1;
            // límites aproximados con la caja girada
            let half = [size.x / 2.0, size.y / 2.0, size.z / 2.0];
            for a in 0..3 {
                let ext = (rot[a * 3].abs() * half[0]) + (rot[a * 3 + 1].abs() * half[1]) + (rot[a * 3 + 2].abs() * half[2]);
                m.bounds_min[a] = m.bounds_min[a].min(pos[a] - ext);
                m.bounds_max[a] = m.bounds_max[a].max(pos[a] + ext);
            }
            m.parts.push(RbxPart { path: path.clone(), class: class.clone(), shape, size: [size.x, size.y, size.z], position: pos, rotation: rot, color: color(i),
                material, transparency: f32p(i, "Transparency").unwrap_or(0.0), mesh_id: text(i, "MeshId").or_else(|| text(i, "MeshContent")).filter(|s| !s.is_empty()) });
        }
        if SCRIPT_CLASSES.contains(&class.as_str()) {
            m.scripts.push(RbxScript { path: path.clone(), class: class.clone(), source: text(i, "Source").unwrap_or_default() });
        }
        if class == "KeyframeSequence" {
            let mut anim = RbxAnimation { path: path.clone(), keyframes: 0, length: 0.0, looped: matches!(i.properties.get(&"Loop".into()), Some(Variant::Bool(true))), priority: enum_val(i, "Priority"), poses: vec![] };
            for kr in i.children() {
                let Some(k) = dom.get_by_ref(*kr) else { continue };
                if k.class.as_str() != "Keyframe" { continue; }
                anim.keyframes += 1;
                anim.length = anim.length.max(f32p(k, "Time").unwrap_or(0.0));
                for d in dom.descendants_of(*kr) {
                    if d.class.as_str() == "Pose" && d.name != "HumanoidRootPart" && !anim.poses.contains(&d.name) { anim.poses.push(d.name.clone()); }
                }
            }
            m.animations.push(anim);
        }
        let children = i.children().iter().filter_map(|r| dom.get_by_ref(*r)).map(|c| walk(dom, c, &path, m)).collect();
        RbxNode { name: i.name.clone(), class, children }
    }
    let root = dom.root();
    m.tree = root.children().iter().filter_map(|r| dom.get_by_ref(*r)).map(|c| walk(dom, c, "", &mut m)).collect();
    if m.parts.is_empty() { m.bounds_min = [0.0; 3]; m.bounds_max = [0.0; 3]; }
    m
}

/// Convierte RBXMX (o RBXM) a RBXM binario, el formato que acepta SerializationService en Studio.
pub fn to_rbxm(bytes: &[u8], format: &str) -> Result<Vec<u8>, String> {
    if format == "rbxm" { return Ok(bytes.to_vec()); }
    let dom = rbx_xml::from_reader_default(bytes).map_err(|e| format!("RBXMX no válido: {e}"))?;
    let mut out = vec![];
    rbx_binary::to_writer(&mut out, &dom, dom.root().children()).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Metadatos resumidos para guardar en la ficha del recurso.
pub fn summary(m: &RbxModel) -> serde_json::Value {
    let dims: Vec<f64> = (0..3).map(|a| (((m.bounds_max[a] - m.bounds_min[a]) as f64) * 100.0).round() / 100.0).collect();
    serde_json::json!({
        "pieces": m.parts.len(),
        "instances": m.instance_count,
        "dimensions_studs": dims,
        "materials": m.materials.keys().collect::<Vec<_>>(),
        "scripts": m.scripts.iter().map(|s| format!("{} ({})", s.path, s.class)).collect::<Vec<_>>(),
        "mesh_parts_without_geometry": m.parts.iter().filter(|p| p.shape == "mesh").count(),
        "animations": m.animations.iter().map(|a| serde_json::json!({ "path": a.path, "keyframes": a.keyframes, "length_s": a.length, "looped": a.looped, "poses": a.poses })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const XML: &str = r#"<roblox version="4">
  <Item class="Model" referent="RBX0"><Properties><string name="Name">Lamp</string></Properties>
    <Item class="Part" referent="RBX1"><Properties>
      <string name="Name">Base</string>
      <CoordinateFrame name="CFrame"><X>0</X><Y>0.5</Y><Z>0</Z><R00>1</R00><R01>0</R01><R02>0</R02><R10>0</R10><R11>1</R11><R12>0</R12><R20>0</R20><R21>0</R21><R22>1</R22></CoordinateFrame>
      <Vector3 name="size"><X>4</X><Y>1</Y><Z>4</Z></Vector3>
      <Color3uint8 name="Color3uint8">4294901760</Color3uint8>
      <token name="Material">1088</token>
    </Properties></Item>
    <Item class="Part" referent="RBX2"><Properties>
      <string name="Name">Bulb</string>
      <CoordinateFrame name="CFrame"><X>0</X><Y>3</Y><Z>0</Z><R00>1</R00><R01>0</R01><R02>0</R02><R10>0</R10><R11>1</R11><R12>0</R12><R20>0</R20><R21>0</R21><R22>1</R22></CoordinateFrame>
      <Vector3 name="size"><X>2</X><Y>2</Y><Z>2</Z></Vector3>
      <token name="shape">0</token>
      <token name="Material">288</token>
    </Properties></Item>
    <Item class="Script" referent="RBX3"><Properties><string name="Name">Toggle</string><ProtectedString name="Source"><![CDATA[print("on")]]></ProtectedString></Properties></Item>
  </Item>
</roblox>"#;

    #[test]
    fn parses_rbxmx_and_roundtrips_binary() {
        let m = parse(XML.as_bytes(), "rbxmx").unwrap();
        assert_eq!(m.parts.len(), 2);
        assert_eq!(m.parts[0].material, "Metal");
        assert_eq!(m.parts[0].color, [255, 0, 0]);
        assert_eq!(m.parts[1].shape, "ball");
        assert_eq!(m.scripts[0].source, "print(\"on\")");
        assert_eq!(m.tree[0].name, "Lamp");
        assert_eq!(m.bounds_max[1], 4.0);
        let s = summary(&m);
        assert_eq!(s["pieces"], 2);
        assert_eq!(s["dimensions_studs"][1], 4.0);

        // binario: reescribimos el DOM como .rbxm y lo volvemos a leer
        let dom = rbx_xml::from_reader_default(XML.as_bytes()).unwrap();
        let mut buf = vec![];
        rbx_binary::to_writer(&mut buf, &dom, dom.root().children()).unwrap();
        let mb = parse(&buf, "rbxm").unwrap();
        assert_eq!(mb.parts.len(), 2);
        assert_eq!(mb.scripts[0].source, "print(\"on\")");
    }

    #[test]
    fn parses_keyframe_sequence() {
        let xml = r#"<roblox version="4"><Item class="KeyframeSequence" referent="A"><Properties><string name="Name">Wave</string><bool name="Loop">true</bool></Properties>
          <Item class="Keyframe" referent="B"><Properties><string name="Name">K0</string><float name="Time">0</float></Properties>
            <Item class="Pose" referent="C"><Properties><string name="Name">HumanoidRootPart</string></Properties>
              <Item class="Pose" referent="D"><Properties><string name="Name">RightUpperArm</string></Properties></Item></Item></Item>
          <Item class="Keyframe" referent="E"><Properties><string name="Name">K1</string><float name="Time">1.25</float></Properties></Item>
        </Item></roblox>"#;
        let m = parse(xml.as_bytes(), "rbxmx").unwrap();
        assert_eq!(m.animations.len(), 1);
        let a = &m.animations[0];
        assert_eq!((a.keyframes, a.length, a.looped), (2, 1.25, true));
        assert_eq!(a.poses, vec!["RightUpperArm".to_string()]);
    }
}
