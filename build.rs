use progenitor::Generator;
use std::{
    env,
    fs::{self, File},
    path::Path,
};
use syn::{
    ItemEnum,
    visit_mut::{self, VisitMut},
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
    let mut ast = syn::parse2(tokens).unwrap();
    AddClapValueEnum.visit_file_mut(&mut ast);

    let mut out_file = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push(format!("codegen_{name}.rs"));

    let content = prettyplease::unparse(&ast);
    fs::write(out_file, content).unwrap();
}

struct AddClapValueEnum;

impl VisitMut for AddClapValueEnum {
    fn visit_item_enum_mut(&mut self, node: &mut ItemEnum) {
        if let "SimulationsV6ListStatus"
        | "SimulationsV7ListStatus"
        | "SimulationQuality"
        | "ProjectStatus" = node.ident.to_string().as_str()
        {
            node.attrs
                .push(syn::parse_quote! { #[derive(clap::ValueEnum)] });
        }

        visit_mut::visit_item_enum_mut(self, node);
    }
}
