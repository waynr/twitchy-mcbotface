fn main() {
    tonic_build::compile_protos("proto/twitch.proto").unwrap();
}
