fn main() {
    anchor_codegen::ConfigBuilder::new()
        .entry("src/main.rs")
        .set_version("KLooper 0.1")
        .set_build_versions("Rust: 1.76")
        .build()
}
