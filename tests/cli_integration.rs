use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "fast_compress_test_{label}_{}_{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("failed to create temp test dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_fast-compress")
}

fn make_binary_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    // All 256 byte values (includes invalid UTF-8 sequences and NUL bytes).
    data.extend(0u8..=255);
    // Long zero run (tests RLE and overlapping LZ77 matches).
    data.extend_from_slice(&[0x00; 2048]);
    // Structured repeated records.
    for i in 0..150u8 {
        data.extend_from_slice(b"\x7FELF\x02\x01\x01\x00RECORD_HEADER_");
        data.push(i);
        data.extend_from_slice(&[0xFF, 0xFE, 0xFD, 0x00, 0x80, 0x99]);
    }
    data
}

#[test]
fn single_binary_file_roundtrips_across_all_algorithms() {
    let tmp = TempDir::new("binary_algos");
    let src_file = tmp.path().join("sample.bin");
    let original = make_binary_fixture();
    fs::write(&src_file, &original).unwrap();

    for algo in ["auto", "lz77", "rle", "store"] {
        let archive = tmp.path().join(format!("out_{algo}.cmpr"));
        let extract_dir = tmp.path().join(format!("extract_{algo}"));

        let comp_out = Command::new(bin_path())
            .args(["-p"])
            .arg(&src_file)
            .args(["-c", "-a", algo, "-d"])
            .arg(&archive)
            .output()
            .unwrap();
        assert!(
            comp_out.status.success(),
            "compress ({algo}) failed: {}",
            String::from_utf8_lossy(&comp_out.stderr)
        );

        let ext_out = Command::new(bin_path())
            .args(["-p"])
            .arg(&archive)
            .args(["-e", "-d"])
            .arg(&extract_dir)
            .output()
            .unwrap();
        assert!(
            ext_out.status.success(),
            "extract ({algo}) failed: {}",
            String::from_utf8_lossy(&ext_out.stderr)
        );

        let restored = fs::read(extract_dir.join("sample.bin")).unwrap();
        assert_eq!(restored, original, "mismatch for algorithm {algo}");
    }
}

#[test]
fn empty_file_roundtrip() {
    let tmp = TempDir::new("empty_file");
    let src_file = tmp.path().join("empty.dat");
    let archive = tmp.path().join("empty.cmpr");
    let extract_dir = tmp.path().join("out");

    fs::write(&src_file, []).unwrap();

    let comp = Command::new(bin_path())
        .args(["-p"])
        .arg(&src_file)
        .args(["-c", "-d"])
        .arg(&archive)
        .output()
        .unwrap();
    assert!(comp.status.success());

    let ext = Command::new(bin_path())
        .args(["-p"])
        .arg(&archive)
        .args(["-e", "-d"])
        .arg(&extract_dir)
        .output()
        .unwrap();
    assert!(ext.status.success());

    let restored = fs::read(extract_dir.join("empty.dat")).unwrap();
    assert!(restored.is_empty());
}

#[test]
fn nested_directory_tree_compress_list_and_extract() {
    let tmp = TempDir::new("nested_tree");
    let tree = tmp.path().join("my_project");
    let sub_a = tree.join("assets").join("bin");
    let empty_sub = tree.join("empty_dir");

    fs::create_dir_all(&sub_a).unwrap();
    fs::create_dir_all(&empty_sub).unwrap();

    let text_bytes = b"Linea 1\nLinea 2\nLinea 3\n".repeat(40);
    let bin_bytes = make_binary_fixture();

    fs::write(tree.join("notes.txt"), &text_bytes).unwrap();
    fs::write(sub_a.join("blob.bin"), &bin_bytes).unwrap();
    fs::write(tree.join("zero.empty"), []).unwrap();

    let archive = tmp.path().join("tree.cmpr");
    let extract_dir = tmp.path().join("restored");

    let comp = Command::new(bin_path())
        .args(["-p"])
        .arg(&tree)
        .args(["-c", "-d"])
        .arg(&archive)
        .output()
        .unwrap();
    assert!(
        comp.status.success(),
        "compress failed: {}",
        String::from_utf8_lossy(&comp.stderr)
    );

    let list = Command::new(bin_path())
        .args(["-p"])
        .arg(&archive)
        .args(["-l"])
        .output()
        .unwrap();
    assert!(list.status.success());
    let list_stdout = String::from_utf8_lossy(&list.stdout);
    assert!(list_stdout.contains("my_project/notes.txt"));
    assert!(list_stdout.contains("my_project/assets/bin/blob.bin"));
    assert!(list_stdout.contains("my_project/empty_dir/"));
    assert!(list_stdout.contains("my_project/zero.empty"));

    let ext = Command::new(bin_path())
        .args(["-p"])
        .arg(&archive)
        .args(["-e", "-d"])
        .arg(&extract_dir)
        .output()
        .unwrap();
    assert!(
        ext.status.success(),
        "extract failed: {}",
        String::from_utf8_lossy(&ext.stderr)
    );

    let root = extract_dir.join("my_project");
    assert!(root.join("empty_dir").is_dir());
    assert_eq!(fs::read(root.join("notes.txt")).unwrap(), text_bytes);
    assert_eq!(
        fs::read(root.join("assets").join("bin").join("blob.bin")).unwrap(),
        bin_bytes
    );
    assert!(fs::read(root.join("zero.empty")).unwrap().is_empty());
}

#[test]
fn self_inclusion_prevention_when_archive_is_inside_source_dir() {
    let tmp = TempDir::new("self_exclude");
    let src_dir = tmp.path().join("bundle");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("hello.txt"), b"hello world!").unwrap();

    let archive_inside = src_dir.join("archive.cmpr");
    let extract_dir = tmp.path().join("out");

    let comp = Command::new(bin_path())
        .args(["-p"])
        .arg(&src_dir)
        .args(["-c", "-d"])
        .arg(&archive_inside)
        .output()
        .unwrap();
    assert!(
        comp.status.success(),
        "compress failed: {}",
        String::from_utf8_lossy(&comp.stderr)
    );

    let list = Command::new(bin_path())
        .args(["-p"])
        .arg(&archive_inside)
        .args(["-l"])
        .output()
        .unwrap();
    assert!(list.status.success());
    let listing = String::from_utf8_lossy(&list.stdout);
    assert!(listing.contains("bundle/hello.txt"));
    assert!(!listing.contains("archive.cmpr"));

    let ext = Command::new(bin_path())
        .args(["-p"])
        .arg(&archive_inside)
        .args(["-e", "-d"])
        .arg(&extract_dir)
        .output()
        .unwrap();
    assert!(ext.status.success());
    assert!(!extract_dir.join("bundle").join("archive.cmpr").exists());
    assert_eq!(
        fs::read(extract_dir.join("bundle").join("hello.txt")).unwrap(),
        b"hello world!"
    );
}

#[test]
fn detects_corrupted_payload_and_bad_magic_header() {
    let tmp = TempDir::new("corruption");
    let src_file = tmp.path().join("payload.bin");
    fs::write(&src_file, b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap();

    let archive = tmp.path().join("good.cmpr");
    let comp = Command::new(bin_path())
        .args(["-p"])
        .arg(&src_file)
        .args(["-c", "-a", "store", "-d"])
        .arg(&archive)
        .output()
        .unwrap();
    assert!(comp.status.success());

    // Flip last byte inside the stored payload to trigger CRC-32 / decompression failure.
    let mut bytes = fs::read(&archive).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    let corrupt_archive = tmp.path().join("corrupt.cmpr");
    fs::write(&corrupt_archive, &bytes).unwrap();

    let ext_corrupt = Command::new(bin_path())
        .args(["-p"])
        .arg(&corrupt_archive)
        .args(["-e", "-d"])
        .arg(tmp.path().join("out_corrupt"))
        .output()
        .unwrap();
    assert!(!ext_corrupt.status.success());
    let stderr = String::from_utf8_lossy(&ext_corrupt.stderr);
    assert!(
        stderr.contains("CRC") || stderr.contains("mismatch") || stderr.contains("error"),
        "unexpected stderr: {stderr}"
    );

    // Bad magic header file.
    let bad_magic = tmp.path().join("bad_magic.cmpr");
    fs::write(&bad_magic, b"NOT_A_VALID_ARCHIVE_AT_ALL").unwrap();
    let ext_bad = Command::new(bin_path())
        .args(["-p"])
        .arg(&bad_magic)
        .args(["-e", "-d"])
        .arg(tmp.path().join("out_bad"))
        .output()
        .unwrap();
    assert!(!ext_bad.status.success());
}

#[test]
fn legacy_typo_cli_aliases_work() {
    let tmp = TempDir::new("legacy_aliases");
    let src_file = tmp.path().join("data.bin");
    let payload = b"\x00\xFF\x01\x02hello_legacy_aliases";
    fs::write(&src_file, payload).unwrap();

    let archive = tmp.path().join("legacy.cmpr");
    let extract_dir = tmp.path().join("legacy_out");

    let comp = Command::new(bin_path())
        .args(["-p"])
        .arg(&src_file)
        .args(["-c", "--destionation"])
        .arg(&archive)
        .output()
        .unwrap();
    assert!(
        comp.status.success(),
        "compress with --destionation failed: {}",
        String::from_utf8_lossy(&comp.stderr)
    );

    let ext = Command::new(bin_path())
        .args(["-p"])
        .arg(&archive)
        .args(["--exctract", "--destionation"])
        .arg(&extract_dir)
        .output()
        .unwrap();
    assert!(
        ext.status.success(),
        "extract with --exctract failed: {}",
        String::from_utf8_lossy(&ext.stderr)
    );

    assert_eq!(fs::read(extract_dir.join("data.bin")).unwrap(), payload);
}
