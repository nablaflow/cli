fn main() {
    cynic_codegen::register_schema("aerocloud")
        .from_sdl_file("schemas/aerocloud.schema.graphql")
        .expect("failed to register aerocloud schema");
}
