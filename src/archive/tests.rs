//! Unit tests for [`super`].

use super::*;
use std::io::Write;

fn make_zip(path: &Path) {
    let mut w = zip::ZipWriter::new(File::create(path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    w.add_directory("docs/", opts).unwrap();
    w.start_file("docs/readme.txt", opts).unwrap();
    w.write_all(b"hello zip").unwrap();
    w.start_file("top.txt", opts).unwrap();
    w.write_all(b"x").unwrap();
    w.finish().unwrap();
}

fn make_tar(path: &Path, gz: bool) {
    let file = File::create(path).unwrap();
    let writer: Box<dyn Write> = if gz {
        Box::new(flate2::write::GzEncoder::new(
            file,
            flate2::Compression::default(),
        ))
    } else {
        Box::new(file)
    };
    let mut b = tar::Builder::new(writer);
    let data = b"hello tar";
    let mut h = tar::Header::new_gnu();
    h.set_size(data.len() as u64);
    h.set_mode(0o644);
    h.set_cksum();
    b.append_data(&mut h, "dir/file.txt", &data[..]).unwrap();
    b.finish().unwrap();
}

#[test]
fn detects_formats_case_insensitively() {
    for (name, want) in [
        ("a.zip", Some(ArchiveFormat::Zip)),
        ("A.ZIP", Some(ArchiveFormat::Zip)),
        ("a.tar", Some(ArchiveFormat::Tar)),
        ("a.tar.gz", Some(ArchiveFormat::TarGz)),
        ("a.tgz", Some(ArchiveFormat::TarGz)),
        ("a.txt", None),
        ("gz", None),
    ] {
        assert_eq!(ArchiveFormat::detect(Path::new(name)), want, "{name}");
    }
}

#[test]
fn default_destination_strips_archive_extension() {
    assert_eq!(
        default_destination(Path::new("/x/foo.tar.gz")),
        PathBuf::from("/x/foo")
    );
    assert_eq!(
        default_destination(Path::new("/x/foo.zip")),
        PathBuf::from("/x/foo")
    );
    assert_eq!(
        default_destination(Path::new("/x/foo.TGZ")),
        PathBuf::from("/x/foo")
    );
    assert_eq!(
        default_destination(Path::new("/x/.zip")),
        PathBuf::from("/x/extracted")
    );
}

#[test]
fn lists_zip_contents() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("a.zip");
    make_zip(&p);
    let l = list(&p).unwrap();
    assert_eq!(l.format, ArchiveFormat::Zip);
    assert_eq!(l.total_entries, 3);
    assert_eq!(l.total_size, 10);
    assert!(l.entries.iter().any(|e| e.name == "docs/" && e.is_dir));
    assert!(l
        .entries
        .iter()
        .any(|e| e.name == "docs/readme.txt" && e.size == 9));
}

#[test]
fn lists_tar_and_targz_contents() {
    let tmp = tempfile::tempdir().unwrap();
    for (name, gz) in [("a.tar", false), ("a.tar.gz", true)] {
        let p = tmp.path().join(name);
        make_tar(&p, gz);
        let l = list(&p).unwrap();
        assert_eq!(l.total_entries, 1, "{name}");
        assert_eq!(l.entries[0].name, "dir/file.txt");
        assert_eq!(l.entries[0].size, 9);
    }
}

#[test]
fn list_rejects_unsupported_and_corrupt_files() {
    let tmp = tempfile::tempdir().unwrap();
    let txt = tmp.path().join("a.txt");
    std::fs::write(&txt, "x").unwrap();
    assert!(list(&txt).is_err());
    let bad = tmp.path().join("bad.zip");
    std::fs::write(&bad, "not a zip").unwrap();
    assert!(list(&bad).is_err());
}

#[test]
fn extracts_zip_tar_and_targz() {
    let tmp = tempfile::tempdir().unwrap();
    let z = tmp.path().join("a.zip");
    make_zip(&z);
    let out = tmp.path().join("zout");
    assert_eq!(extract(&z, &out).unwrap(), 3);
    assert_eq!(
        std::fs::read_to_string(out.join("docs/readme.txt")).unwrap(),
        "hello zip"
    );

    for (name, gz) in [("a.tar", false), ("a.tgz", true)] {
        let t = tmp.path().join(name);
        make_tar(&t, gz);
        let out = tmp.path().join(format!("out-{name}"));
        assert_eq!(extract(&t, &out).unwrap(), 1);
        assert_eq!(
            std::fs::read_to_string(out.join("dir/file.txt")).unwrap(),
            "hello tar"
        );
    }
}

#[test]
fn tar_path_traversal_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("evil.tar");
    {
        let mut b = tar::Builder::new(File::create(&p).unwrap());
        let data = b"pwn";
        let mut h = tar::Header::new_gnu();
        h.set_size(3);
        h.set_mode(0o644);
        // Write the raw name (the safe `append_data` would refuse "..").
        let name = b"../escaped.txt";
        h.as_old_mut().name[..name.len()].copy_from_slice(name);
        h.set_cksum();
        b.append(&h, &data[..]).unwrap();
        b.finish().unwrap();
    }
    let out = tmp.path().join("out");
    assert!(extract(&p, &out).is_err());
    assert!(!tmp.path().join("escaped.txt").exists());
}

#[test]
fn zip_path_traversal_does_not_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("evil.zip");
    {
        let mut w = zip::ZipWriter::new(File::create(&p).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        w.start_file("../escaped.txt", opts).unwrap();
        w.write_all(b"pwn").unwrap();
        w.finish().unwrap();
    }
    let out = tmp.path().join("out");
    let _ = extract(&p, &out);
    assert!(!tmp.path().join("escaped.txt").exists());
}

#[test]
fn zip_listing_reports_sizes_and_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("a.zip");
    make_zip(&p);
    let l = list(&p).unwrap();
    assert_eq!(l.total_entries, 3);
    assert_eq!(l.total_size, 10);
}

#[test]
fn failed_extraction_removes_new_destination() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("bad.zip");
    std::fs::write(&p, b"not a zip").unwrap();
    let out = tmp.path().join("out");
    assert!(extract(&p, &out).is_err());
    assert!(!out.exists());
}

#[test]
fn failed_extraction_keeps_existing_destination() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("bad.zip");
    std::fs::write(&p, b"not a zip").unwrap();
    let out = tmp.path().join("out");
    std::fs::create_dir(&out).unwrap();
    assert!(extract(&p, &out).is_err());
    assert!(out.exists());
}
