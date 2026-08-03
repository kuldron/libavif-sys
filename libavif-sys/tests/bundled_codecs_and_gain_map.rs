//! Guards the two ways the libavif 1.3.0 bump can break silently.
//!
//! 1. **Codecs disabled.** From libavif 1.2.0 the `AVIF_CODEC_*` cmake options are
//!    tri-state (`OFF` / `LOCAL` / `SYSTEM`). The legacy `=1` this crate used to
//!    pass is none of those, so `check_avif_option` leaves the codec off and the
//!    build still SUCCEEDS -- yielding a libavif that cannot encode or decode
//!    anything, failing only at runtime. A round trip is the only honest check.
//!
//! 2. **Stale bindings.** `avifImage` grew `properties`, `numProperties` and
//!    `gainMap` after 1.0.0, and `gainMap` is the last field. Reading it proves the
//!    generated layout agrees with the C library we linked; a 1.0.4-era binding
//!    would read past the end of the struct.

use libavif_sys as sys;

#[test]
fn links_libavif_1_3_x() {
    let version = unsafe { std::ffi::CStr::from_ptr(sys::avifVersion()) }
        .to_string_lossy()
        .into_owned();
    assert!(
        version.starts_with("1.3."),
        "expected libavif 1.3.x, linked {version}"
    );
}

#[test]
fn gain_map_api_is_present_and_the_struct_layout_agrees() {
    unsafe {
        let img = sys::avifImageCreate(16, 16, 8, sys::AVIF_PIXEL_FORMAT_YUV444);
        assert!(!img.is_null(), "avifImageCreate returned null");
        // The last field of the struct: null for a plain image, but reachable at
        // all only if the bindings match the linked library.
        assert!(
            (*img).gainMap.is_null(),
            "a freshly created image must have no gain map"
        );
        assert_eq!((*img).numProperties, 0);
        sys::avifImageDestroy(img);

        // Present since 1.2.0; taking its address proves the symbol links rather
        // than merely existing in the header. The signature is the one the whole
        // bump is for: (base, alternate) -> gain map.
        let compute: unsafe extern "C" fn(
            *const sys::avifImage,
            *const sys::avifImage,
            *mut sys::avifGainMap,
            *mut sys::avifDiagnostics,
        ) -> u32 = sys::avifImageComputeGainMap;
        // Binding it to that type is the assertion -- it fails to compile if the
        // signature drifts. black_box keeps the reference from being optimised
        // away, so the symbol must actually resolve at link time.
        std::hint::black_box(compute);
    }
}

#[test]
fn round_trips_through_rav1e_and_dav1d() {
    unsafe {
        let img = sys::avifImageCreate(16, 16, 8, sys::AVIF_PIXEL_FORMAT_YUV444);
        assert!(!img.is_null());

        let mut rgb: sys::avifRGBImage = std::mem::zeroed();
        sys::avifRGBImageSetDefaults(&mut rgb, img);
        rgb.format = sys::AVIF_RGB_FORMAT_RGBA;
        rgb.depth = 8;
        assert_eq!(
            sys::avifRGBImageAllocatePixels(&mut rgb),
            sys::AVIF_RESULT_OK
        );
        for y in 0..16usize {
            let row = rgb.pixels.add(y * rgb.rowBytes as usize);
            for x in 0..16usize * 4 {
                *row.add(x) = ((x * 7 + y * 3) % 256) as u8;
            }
        }
        assert_eq!(sys::avifImageRGBToYUV(img, &rgb), sys::AVIF_RESULT_OK);
        sys::avifRGBImageFreePixels(&mut rgb);

        let encoder = sys::avifEncoderCreate();
        assert!(!encoder.is_null());
        (*encoder).speed = 10;
        (*encoder).quality = 60;
        let mut out: sys::avifRWData = std::mem::zeroed();
        assert_eq!(
            sys::avifEncoderWrite(encoder, img, &mut out),
            sys::AVIF_RESULT_OK,
            "rav1e encode failed -- codec likely not compiled in"
        );
        assert!(out.size > 0, "encoder produced no bytes");

        let decoder = sys::avifDecoderCreate();
        assert!(!decoder.is_null());
        assert_eq!(
            sys::avifDecoderSetIOMemory(decoder, out.data, out.size),
            sys::AVIF_RESULT_OK
        );
        assert_eq!(sys::avifDecoderParse(decoder), sys::AVIF_RESULT_OK);
        assert_eq!(
            sys::avifDecoderNextImage(decoder),
            sys::AVIF_RESULT_OK,
            "dav1d decode failed -- codec likely not compiled in"
        );
        assert_eq!((*(*decoder).image).width, 16);
        assert_eq!((*(*decoder).image).height, 16);

        sys::avifDecoderDestroy(decoder);
        sys::avifRWDataFree(&mut out);
        sys::avifEncoderDestroy(encoder);
        sys::avifImageDestroy(img);
    }
}
