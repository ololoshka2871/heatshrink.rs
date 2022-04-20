fn main() {
    let src = [
        "../heatshrink-dist/heatshrink_decoder.c",
        "../heatshrink-dist/heatshrink_encoder.c",
    ];
    let mut builder = cc::Build::new();
    let builder = builder
        .files(src.iter())
        .include("../heatshrink")
        .opt_level_str("s")
        .define("HEATSHRINK_DYNAMIC_ALLOC", Some("0"))
        //.define("HEATSHRINK_DEBUGGING_LOGS", Some("1"))
        ;
    #[cfg(not(target_os = "windows"))]
    let builder = builder.flag("-Wno-implicit-fallthrough");

    builder.compile("heatshrink");
}