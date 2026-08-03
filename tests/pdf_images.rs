//! End-to-end checks for PDF loading: page-aware text plus pixel-exact
//! extraction of embedded raster images.
//!
//! The fixture PDF is built here with `lopdf` rather than checked in, so the
//! expected pixels are stated next to the bytes that produce them.

use std::path::PathBuf;

use flerp::pdf_doc;
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};

/// 2x2 true-colour image: red, green / blue, white.
const RGB_PIXELS: [u8; 12] = [
    255, 0, 0, /**/ 0, 255, 0, //
    0, 0, 255, /**/ 255, 255, 255,
];

/// 4x2 one-bit stencil. Each row is padded to a full byte, so only the top
/// nibble of each byte carries pixels.
const BILEVEL_ROWS: [u8; 2] = [0b1010_0000, 0b0101_0000];

/// A 4-entry RGB palette addressed by the 2x2 indexed image below.
const PALETTE: [u8; 12] = [
    0, 0, 0, /**/ 255, 0, 0, //
    0, 255, 0, /**/ 0, 0, 255,
];
/// 2x2 at 4 bits per sample, so one row fits in one byte. Samples are packed
/// most-significant nibble first: row 0 selects palette entries 0 and 1, row 1
/// selects 2 and 3.
const PALETTE_INDICES: [u8; 2] = [0b0000_0001, 0b0010_0011];

fn image_stream(dict: lopdf::Dictionary, content: Vec<u8>) -> Stream {
    let mut stream = Stream::new(dict, content);
    stream.compress().expect("image stream should compress");
    stream
}

/// Build a two-page PDF: text on both pages, three images on page two.
fn write_fixture(path: &PathBuf) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let rgb_id = doc.add_object(image_stream(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 2,
            "Height" => 2,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
        },
        RGB_PIXELS.to_vec(),
    ));

    let bilevel_id = doc.add_object(image_stream(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 4,
            "Height" => 2,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 1,
        },
        BILEVEL_ROWS.to_vec(),
    ));

    let indexed_id = doc.add_object(image_stream(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 2,
            "Height" => 2,
            "ColorSpace" => Object::Array(vec![
                Object::Name(b"Indexed".to_vec()),
                Object::Name(b"DeviceRGB".to_vec()),
                Object::Integer(3),
                Object::String(PALETTE.to_vec(), lopdf::StringFormat::Hexadecimal),
            ]),
            "BitsPerComponent" => 4,
        },
        PALETTE_INDICES.to_vec(),
    ));

    let mut page_ids = Vec::new();
    for (index, body) in ["Alpha page text", "Beta page text"].iter().enumerate() {
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 24.into()]),
                Operation::new("Td", vec![72.into(), 700.into()]),
                Operation::new("Tj", vec![Object::string_literal(*body)]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id =
            doc.add_object(Stream::new(dictionary! {}, content.encode().expect("content")));

        // Only the second page carries images.
        let xobjects = if index == 1 {
            dictionary! {
                "Im0" => Object::Reference(rgb_id),
                "Im1" => Object::Reference(bilevel_id),
                "Im2" => Object::Reference(indexed_id),
            }
        } else {
            dictionary! {}
        };

        page_ids.push(Object::Reference(doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! { "F1" => Object::Reference(font_id) },
                "XObject" => xobjects,
            },
        })));
    }

    let page_count = page_ids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids,
            "Count" => page_count,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    doc.save(path).expect("fixture PDF should save");
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("flerp-{name}-{}.pdf", std::process::id()));
        write_fixture(&path);
        Self { path }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn reads_every_page_and_tracks_where_each_one_starts() {
    let fixture = Fixture::new("pages");
    let document = pdf_doc::load(fixture.path.to_str().unwrap()).expect("PDF should load");

    assert_eq!(document.page_count(), 2);

    // Both pages' text is present -- the load is complete, not a sample.
    assert!(document.text.contains("Alpha page text"));
    assert!(document.text.contains("Beta page text"));

    // Page starts line up with the joined text.
    let lines: Vec<&str> = document.text.lines().collect();
    for page in &document.pages {
        assert!(
            page.start_line < lines.len(),
            "page {} starts past the end of the text",
            page.number
        );
    }
    let second = &document.pages[1];
    assert!(lines[second.start_line..].join("\n").contains("Beta page text"));

    // Every line maps back to the page it came from.
    assert_eq!(document.page_of_line(0), 0);
    assert_eq!(document.page_of_line(second.start_line), 1);
    assert_eq!(document.page_of_line(usize::MAX), 1);
}

#[test]
fn decodes_embedded_images_to_exact_pixels() {
    let fixture = Fixture::new("images");
    let document = pdf_doc::load(fixture.path.to_str().unwrap()).expect("PDF should load");

    assert!(
        document.image_error.is_none(),
        "image extraction failed: {:?}",
        document.image_error
    );
    assert_eq!(
        document.skipped.len(),
        0,
        "unexpected skips: {:?}",
        document
            .skipped
            .iter()
            .map(|skipped| skipped.reason.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(document.images.len(), 3);

    // All three live on page two, and the page records that.
    assert!(document.images.iter().all(|asset| asset.page == 2));
    assert_eq!(document.pages[0].image_count, 0);
    assert_eq!(document.pages[1].image_count, 3);

    let rgb = document
        .images
        .iter()
        .find(|asset| asset.width == 2 && asset.color_space == "DeviceRGB")
        .expect("the true-colour image should be decoded");
    let pixels = rgb.image.to_rgb8();
    assert_eq!((pixels.width(), pixels.height()), (2, 2));
    assert_eq!(pixels.get_pixel(0, 0).0, [255, 0, 0]);
    assert_eq!(pixels.get_pixel(1, 0).0, [0, 255, 0]);
    assert_eq!(pixels.get_pixel(0, 1).0, [0, 0, 255]);
    assert_eq!(pixels.get_pixel(1, 1).0, [255, 255, 255]);

    // One-bit samples expand to full black and white, and row padding is skipped.
    let bilevel = document
        .images
        .iter()
        .find(|asset| asset.width == 4)
        .expect("the one-bit image should be decoded");
    let gray = bilevel.image.to_luma8();
    assert_eq!((gray.width(), gray.height()), (4, 2));
    let row0: Vec<u8> = (0..4).map(|x| gray.get_pixel(x, 0).0[0]).collect();
    let row1: Vec<u8> = (0..4).map(|x| gray.get_pixel(x, 1).0[0]).collect();
    assert_eq!(row0, vec![255, 0, 255, 0]);
    assert_eq!(row1, vec![0, 255, 0, 255]);

    // Palette indices resolve through the lookup table.
    let indexed = document
        .images
        .iter()
        .find(|asset| asset.color_space == "Indexed")
        .expect("the indexed image should be decoded");
    let palette_pixels = indexed.image.to_rgb8();
    assert_eq!(palette_pixels.get_pixel(0, 0).0, [0, 0, 0]);
    assert_eq!(palette_pixels.get_pixel(1, 0).0, [255, 0, 0]);
    assert_eq!(palette_pixels.get_pixel(0, 1).0, [0, 255, 0]);
    assert_eq!(palette_pixels.get_pixel(1, 1).0, [0, 0, 255]);
}
