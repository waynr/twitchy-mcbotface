use std::path::PathBuf;

fn main() {
    let imports: &[&str] = &[];

    tonic_build::configure()
        .build_server(true)
        .out_dir(&PathBuf::from("./src/pb/"))
        .compile(&["proto/twitch.proto"], &imports)
        .unwrap();
}
