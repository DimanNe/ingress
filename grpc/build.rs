// fn main() -> Result<(), Box<dyn std::error::Error>> {
//    tonic_build::compile_protos("back.proto")?;
//    Ok(())
// }

/// Make generated files available for IDE, by generating them in the source dir:
fn main() {
   let out_path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/generated");
   std::fs::create_dir_all(&out_path).expect("Failed to create directory for generated files");

   tonic_build::configure().out_dir(out_path)
                           .compile(&["back.proto"], &["proto"])
                           .expect("Failed to compile protos");
}
