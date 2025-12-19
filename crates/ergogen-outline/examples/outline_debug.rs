use ergogen_outline::generate_outline_region_from_yaml_str;
use ergogen_export::dxf_geom::dxf_from_region;
use cavalier_contours::polyline::PlineSource;

fn main() {
    let path = std::env::args().nth(1).expect("yaml path");
    let outline = std::env::args().nth(2).expect("outline name");
    let yaml = std::fs::read_to_string(&path).unwrap();
    let region = generate_outline_region_from_yaml_str(&yaml, &outline).unwrap();
    println!("pos={} neg={}", region.pos.len(), region.neg.len());
    for (i, p) in region.pos.iter().enumerate() {
        let mut lines = 0;
        let mut arcs = 0;
        let mut minx = f64::INFINITY;
        let mut miny = f64::INFINITY;
        let mut maxx = f64::NEG_INFINITY;
        let mut maxy = f64::NEG_INFINITY;
        for idx in 0..p.vertex_count() {
            let v = p.at(idx);
            minx = minx.min(v.x);
            miny = miny.min(v.y);
            maxx = maxx.max(v.x);
            maxy = maxy.max(v.y);
            if v.bulge_is_zero() {
                lines += 1;
            } else {
                arcs += 1;
            }
        }
        println!(
            "pos[{i}] vertices={} area={} orientation={:?} segs(line={lines} arc={arcs}) bbox=({minx},{miny})..({maxx},{maxy})",
            p.vertex_count(),
            p.area(),
            p.orientation()
        );
        if std::env::var("DUMP_VERTS").is_ok() {
            for idx in 0..p.vertex_count() {
                let v = p.at(idx);
                println!("  v{idx}: ({},{}) bulge={}", v.x, v.y, v.bulge);
            }
        }
    }
    for (i, p) in region.neg.iter().enumerate() {
        let mut lines = 0;
        let mut arcs = 0;
        let mut minx = f64::INFINITY;
        let mut miny = f64::INFINITY;
        let mut maxx = f64::NEG_INFINITY;
        let mut maxy = f64::NEG_INFINITY;
        for idx in 0..p.vertex_count() {
            let v = p.at(idx);
            minx = minx.min(v.x);
            miny = miny.min(v.y);
            maxx = maxx.max(v.x);
            maxy = maxy.max(v.y);
            if v.bulge_is_zero() {
                lines += 1;
            } else {
                arcs += 1;
            }
        }
        println!(
            "neg[{i}] vertices={} area={} orientation={:?} segs(line={lines} arc={arcs}) bbox=({minx},{miny})..({maxx},{maxy})",
            p.vertex_count(),
            p.area(),
            p.orientation()
        );
        if std::env::var("DUMP_VERTS").is_ok() {
            for idx in 0..p.vertex_count() {
                let v = p.at(idx);
                println!("  v{idx}: ({},{}) bulge={}", v.x, v.y, v.bulge);
            }
        }
    }
    let dxf = dxf_from_region(&region).unwrap();
    let mut line=0; let mut arc=0; let mut circle=0; let mut lw=0;
    for e in dxf.entities {
        match e {
            ergogen_export::dxf::Entity::Line(_) => line+=1,
            ergogen_export::dxf::Entity::Arc(_) => arc+=1,
            ergogen_export::dxf::Entity::Circle(_) => circle+=1,
            ergogen_export::dxf::Entity::LwPolyline(_) => lw+=1,
            ergogen_export::dxf::Entity::Unsupported(_) => {}
        }
    }
    println!("entities line={line} arc={arc} circle={circle} lw={lw}");
}
