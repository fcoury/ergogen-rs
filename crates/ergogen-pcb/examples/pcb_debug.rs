use std::env;
use std::fs;

fn main() {
    let path = env::args().nth(1).expect("yaml path");
    let pcb = env::args().nth(2).expect("pcb name");
    let yaml = fs::read_to_string(&path).expect("read yaml");
    let out = ergogen_pcb::generate_kicad_pcb_from_yaml_str(&yaml, &pcb).expect("generate pcb");
    print!("{out}");
}
