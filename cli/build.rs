fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");
}
