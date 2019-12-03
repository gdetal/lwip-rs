extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {
    let mut config = cc::Build::new();
    if env::var("PROFILE")
        .map(|v| &v[..] == "debug")
        .unwrap_or(false)
    {
        config.define("LWIP_DEBUG", Some("1"));
    }

    let debug = if cfg!(feature = "debug") { "1" } else { "0" };

    config
        .file("ffi/lwip/src/core/def.c")
        .file("ffi/lwip/src/core/inet_chksum.c")
        .file("ffi/lwip/src/core/init.c")
        .file("ffi/lwip/src/core/ip.c")
        .file("ffi/lwip/src/core/ipv4/ip4.c")
        .file("ffi/lwip/src/core/ipv4/ip4_addr.c")
        .file("ffi/lwip/src/core/ipv4/ip4_frag.c")
        .file("ffi/lwip/src/core/ipv6/icmp6.c")
        .file("ffi/lwip/src/core/ipv6/ip6.c")
        .file("ffi/lwip/src/core/ipv6/ip6_addr.c")
        .file("ffi/lwip/src/core/ipv6/ip6_frag.c")
        .file("ffi/lwip/src/core/ipv6/nd6.c")
        .file("ffi/lwip/src/core/mem.c")
        .file("ffi/lwip/src/core/memp.c")
        .file("ffi/lwip/src/core/netif.c")
        .file("ffi/lwip/src/core/pbuf.c")
        .file("ffi/lwip/src/core/raw.c")
        .file("ffi/lwip/src/core/stats.c")
        .file("ffi/lwip/src/core/udp.c")
        .file("ffi/lwip/src/core/tcp.c")
        .file("ffi/lwip/src/core/tcp_in.c")
        .file("ffi/lwip/src/core/tcp_out.c")
        .file("ffi/lwip/src/core/timeouts.c")
        .file("ffi/lwip/src/api/tcpip.c")
        .file("ffi/lwip/src/api/api_lib.c")
        .file("ffi/lwip/src/api/api_msg.c")
        .file("ffi/lwip/src/api/netbuf.c")
        .file("ffi/lwip/src/api/err.c")
        .file("ffi/lwip/src/api/netifapi.c")
        .file("ffi/lwip/contrib/addons/ipv6_static_routing/ip6_route_table.c")
        .file("ffi/src/tcpip_init.c")
        .file("ffi/src/sys.c")
        .include("ffi/src")
        .include("ffi/lwip/contrib/ports/unix")
        .include("ffi/lwip/contrib/ports/unix/port/include")
        .include("ffi/lwip/src/include")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .define("FEATURE_DEBUG", debug)
        .compile("liblwip.a");

    println!("cargo:rustc-link-lib=static=lwip");

    let bindings = bindgen::Builder::default()
        .header("ffi/lwip/src/include/lwip/init.h")
        .header("ffi/lwip/src/include/lwip/raw.h")
        .header("ffi/lwip/src/include/lwip/udp.h")
        .header("ffi/lwip/src/include/lwip/tcp.h")
        .header("ffi/lwip/src/include/lwip/tcpip.h")
        .header("ffi/lwip/src/include/lwip/api.h")
        .header("ffi/lwip/src/include/lwip/netifapi.h")
        .header("ffi/src/tcpip_init.c")
        .clang_arg("-Iffi/lwip/src/include")
        .clang_arg("-Iffi/lwip/contrib/ports/unix/port/include")
        .clang_arg("-Iffi/src")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .derive_debug(true)
        .impl_debug(true)
        .whitelist_function("ffi/lwip_init")
        .whitelist_function("tcpip_.*")
        .whitelist_function("netconn_.*")
        .whitelist_function("netifapi_.*")
        .whitelist_function("raw_.*")
        .whitelist_function("udp_.*")
        .whitelist_function("tcp_.*")
        .whitelist_function("pbuf_.*")
        .whitelist_function("netif_.*")
        .whitelist_function("netbuf_.*")
        .whitelist_function("err_.*")
        .whitelist_type("err_enum_t")
        .whitelist_type("err_t")
        .whitelist_type("lwip_ip_addr_type")
        .rustified_enum("err_enum_t")
        .rustified_enum("pbuf_layer")
        .rustified_enum("pbuf_type")
        .rustified_enum("netconn_type")
        .rustified_enum("netconn_state")
        .rustified_enum("netconn_evt")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
