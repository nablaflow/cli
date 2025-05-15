use progenitor::Generator;
use std::{
    env,
    fs::{self, File},
    path::Path,
};

fn main() {
    generate("aerocloud");
}

fn generate(name: &str) {
    let src = format!("schemas/{name}.json");
    println!("cargo:rerun-if-changed={src}");

    let file = File::open(src).unwrap();
    let spec = serde_json::from_reader(file).unwrap();
    let mut generator = Generator::default();

    let tokens = generator.generate_tokens(&spec).unwrap();
    let ast = syn::parse2(tokens).unwrap();
    let content = prettyplease::unparse(&ast);

    let mut out_file = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push(format!("codegen_{name}.rs"));

    fs::write(out_file, content).unwrap();
}
