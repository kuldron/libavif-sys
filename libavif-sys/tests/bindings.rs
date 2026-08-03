//! Checks that the bindings match the libavif that was built and linked.
//!
//! Both failures these cover are silent. A codec can be disabled by a cmake
//! option libavif no longer recognises, in a build that still succeeds; and the
//! generated bindings can describe a different `avifImage` layout than the
//! library exposes, because `src/ffi.rs` is committed rather than generated at
//! build time and can lag a vendored libavif upgrade.

use libavif_sys as sys;

/// The bindings and the vendored library must describe the same release.
#[test]
fn bindings_match_the_linked_library() {
    let linked = unsafe { std::ffi::CStr::from_ptr(sys::avifVersion()) }
        .to_string_lossy()
        .into_owned();
    let generated = format!(
        "{}.{}.{}",
        sys::AVIF_VERSION_MAJOR,
        sys::AVIF_VERSION_MINOR,
        sys::AVIF_VERSION_PATCH
    );
    assert!(
        linked.starts_with(&generated),
        "bindings were generated against libavif {generated}, but {linked} is linked; \
         regenerate src/ffi.rs using the command in bindings.h"
    );
}

/// `avifImage` gained `properties`, `numProperties` and `gainMap` after 1.0.0.
/// `gainMap` is the final field, so reading it exercises the whole layout:
/// bindings generated against an older libavif would address the wrong offset.
#[test]
fn avif_image_layout_includes_the_fields_added_after_1_0_0() {
    unsafe {
        let image = sys::avifImageCreate(16, 16, 8, sys::AVIF_PIXEL_FORMAT_YUV444);
        assert!(!image.is_null(), "avifImageCreate returned null");
        assert_eq!((*image).numProperties, 0);
        assert!(
            (*image).gainMap.is_null(),
            "a newly created image has no gain map"
        );
        sys::avifImageDestroy(image);
    }
}

/// An encode followed by a decode is the only check that the codecs selected by
/// the enabled features were actually compiled into libavif. A misconfigured
/// codec option produces a library that builds but returns
/// `AVIF_RESULT_NO_CODEC_AVAILABLE` here.
#[cfg(all(
    any(feature = "codec-rav1e", feature = "codec-aom"),
    any(feature = "codec-dav1d", feature = "codec-aom")
))]
#[test]
fn an_image_survives_an_encode_and_decode() {
    unsafe {
        let image = sys::avifImageCreate(16, 16, 8, sys::AVIF_PIXEL_FORMAT_YUV444);
        assert!(!image.is_null());

        let mut rgb: sys::avifRGBImage = std::mem::zeroed();
        sys::avifRGBImageSetDefaults(&mut rgb, image);
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
        assert_eq!(sys::avifImageRGBToYUV(image, &rgb), sys::AVIF_RESULT_OK);
        sys::avifRGBImageFreePixels(&mut rgb);

        let encoder = sys::avifEncoderCreate();
        assert!(!encoder.is_null());
        (*encoder).speed = 10;
        (*encoder).quality = 60;
        let mut encoded: sys::avifRWData = std::mem::zeroed();
        let result = sys::avifEncoderWrite(encoder, image, &mut encoded);
        assert_ne!(
            result,
            sys::AVIF_RESULT_NO_CODEC_AVAILABLE,
            "libavif was built without an encoder; check the AVIF_CODEC_* cmake options"
        );
        assert_eq!(result, sys::AVIF_RESULT_OK);
        assert!(encoded.size > 0);

        let decoder = sys::avifDecoderCreate();
        assert!(!decoder.is_null());
        assert_eq!(
            sys::avifDecoderSetIOMemory(decoder, encoded.data, encoded.size),
            sys::AVIF_RESULT_OK
        );
        assert_eq!(sys::avifDecoderParse(decoder), sys::AVIF_RESULT_OK);
        let result = sys::avifDecoderNextImage(decoder);
        assert_ne!(
            result,
            sys::AVIF_RESULT_NO_CODEC_AVAILABLE,
            "libavif was built without a decoder; check the AVIF_CODEC_* cmake options"
        );
        assert_eq!(result, sys::AVIF_RESULT_OK);
        assert_eq!((*(*decoder).image).width, 16);
        assert_eq!((*(*decoder).image).height, 16);

        sys::avifDecoderDestroy(decoder);
        sys::avifRWDataFree(&mut encoded);
        sys::avifEncoderDestroy(encoder);
        sys::avifImageDestroy(image);
    }
}
