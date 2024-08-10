use std::fs;
use std::fs::File;
use std::io::Write;
use object::{Architecture, BinaryFormat, Endianness, SectionKind};
use object::write::Object;

pub fn pe_to_elf(name: &str, context: u32) {
    let original = fs::read(name).unwrap();
    let mut object = Object::new(BinaryFormat::Elf, Architecture::X86_64, Endianness::Little);

    let section_id = object.add_section(Vec::new(), b".note.tag".to_vec(), SectionKind::Note);

    let note_name = b"BunnyOS\0";
    let note_type = 1u32;

    let abi_version = 1u32;

    let crc: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_BZIP2);
    let checksum = crc.checksum(&original);

    let name_len = note_name.len() as u32;
    let desc_len = 16u32;

    let mut content = Vec::new();
    content.extend_from_slice(&name_len.to_le_bytes());
    content.extend_from_slice(&desc_len.to_le_bytes());
    content.extend_from_slice(&note_type.to_le_bytes());
    content.extend_from_slice(note_name);
    content.extend_from_slice(&0u32.to_le_bytes());
    content.extend_from_slice(&context.to_le_bytes());
    content.extend_from_slice(&abi_version.to_le_bytes());
    content.extend_from_slice(&checksum.to_le_bytes());

    object.section_mut(section_id).set_data(content, 4);

    let section_id = object.add_section(Vec::new(), b".text".to_vec(), SectionKind::Text);
    object.section_mut(section_id).set_data(original, 4);

    let section_id = object.add_section(Vec::new(), b".debug".to_vec(), SectionKind::Debug);
    object.section_mut(section_id).set_data(&[1, 2, 3, 4], 4);

    let mut file = File::create(name).unwrap();
    file.write_all(&object.write().unwrap()).unwrap();
}
