use color_eyre::eyre::eyre;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

const VPK_SIGNATURE: u32 = 0x55AA1234;
const VPK_VERSION_V1: u32 = 1;
const VPK_ENTRY_TERMINATOR: u16 = 0xFFFF;

#[derive(Debug)]
pub struct VPKInfo {
    mission_content: Option<String>,
    #[allow(dead_code)]
    addoninfo_content: Option<String>,
}

impl VPKInfo {
    pub fn new(path: impl AsRef<Path>) -> color_eyre::Result<Self> {
        let path = path.as_ref();
        let file =
            File::open(path).map_err(|e| eyre!("无法打开文件 {}: {}", path.display(), e))?;
        let mut reader = BufReader::new(file);

        // Parse header
        let sig = read_u32(&mut reader)?;
        if sig != VPK_SIGNATURE {
            return Err(eyre!("不是有效的VPK文件 (signature: {:#x})", sig));
        }
        let version = read_u32(&mut reader)?;
        if version != VPK_VERSION_V1 {
            return Err(eyre!("不支持的VPK版本: {}", version));
        }
        let tree_size = read_u32(&mut reader)? as u64;

        let tree_end = reader.stream_position()? + tree_size;

        // Parse tree: extension → directory → filename → entry
        let mut mission_content = None;
        let mut addoninfo_content = None;

        loop {
            let extension = read_string_lossy(&mut reader);
            if extension.is_empty() {
                break;
            }

            loop {
                let directory = read_string_lossy(&mut reader);
                if directory.is_empty() || reader.stream_position()? >= tree_end {
                    break;
                }

                loop {
                    let filename = read_string_lossy(&mut reader);
                    if filename.is_empty() || reader.stream_position()? >= tree_end {
                        break;
                    }

                    // Read VPKDirectoryEntry (18 bytes)
                    let _crc = read_u32(&mut reader)?;
                    let preload_length = read_u16(&mut reader)?;
                    let archive_index = read_u16(&mut reader)?;
                    let entry_offset = read_u32(&mut reader)?;
                    let entry_length = read_u32(&mut reader)?;
                    let terminator = read_u16(&mut reader)?;

                    if terminator != VPK_ENTRY_TERMINATOR {
                        return Err(eyre!("VPK entry terminator 错误"));
                    }

                    // Skip preload data inline in the tree
                    if preload_length > 0 {
                        let mut preload = vec![0u8; preload_length as usize];
                        reader.read_exact(&mut preload)?;
                    }

                    // Check if this is a mission file or addoninfo
                    let is_mission =
                        extension.eq_ignore_ascii_case("txt") && directory.eq_ignore_ascii_case("missions");
                    let is_addoninfo = extension.eq_ignore_ascii_case("txt")
                        && directory.is_empty()
                        && filename.eq_ignore_ascii_case("addoninfo");

                    if mission_content.is_none() && is_mission {
                        mission_content = Some(read_file_data(
                            &mut reader,
                            tree_end,
                            archive_index,
                            entry_offset,
                            entry_length,
                            preload_length,
                        )?);
                    }
                    if addoninfo_content.is_none() && is_addoninfo {
                        addoninfo_content = Some(read_file_data(
                            &mut reader,
                            tree_end,
                            archive_index,
                            entry_offset,
                            entry_length,
                            preload_length,
                        )?);
                    }
                }
            }
        }

        Ok(Self {
            mission_content,
            addoninfo_content,
        })
    }

    pub fn get_mission(&self) -> color_eyre::Result<String> {
        self.mission_content
            .clone()
            .ok_or_else(|| eyre!("该文件没有misson信息"))
    }

    #[allow(dead_code)]
    pub fn get_addoninfo(&self) -> color_eyre::Result<String> {
        self.addoninfo_content
            .clone()
            .ok_or_else(|| eyre!("该文件没有addoninfo信息"))
    }
}

fn read_file_data(
    reader: &mut BufReader<File>,
    tree_end: u64,
    archive_index: u16,
    entry_offset: u32,
    entry_length: u32,
    _preload_length: u16,
) -> color_eyre::Result<String> {
    if entry_length == 0 {
        return Ok(String::new());
    }

    let data_offset = if archive_index == 0x7FFF || archive_index == 0xFF7F || archive_index == 0 {
        // Data follows the directory tree in the same file
        tree_end + entry_offset as u64
    } else {
        return Err(eyre!(
            "不支持多文件VPK (archive_index={})",
            archive_index
        ));
    };

    let saved_pos = reader.stream_position()?;
    reader
        .seek(SeekFrom::Start(data_offset))
        .map_err(|e| eyre!("定位文件数据失败: {}", e))?;

    let mut data = vec![0u8; entry_length as usize];
    reader
        .read_exact(&mut data)
        .map_err(|e| eyre!("读取文件数据失败: {}", e))?;

    reader
        .seek(SeekFrom::Start(saved_pos))
        .map_err(|e| eyre!("恢复读取位置失败: {}", e))?;

    Ok(String::from_utf8_lossy(&data).into_owned())
}

fn read_u32(reader: &mut BufReader<File>) -> color_eyre::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u16(reader: &mut BufReader<File>) -> color_eyre::Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_string_lossy(reader: &mut BufReader<File>) -> String {
    let mut buf = Vec::new();
    loop {
        let mut b = [0u8; 1];
        if reader.read_exact(&mut b).is_err() {
            break;
        }
        if b[0] == 0 {
            break;
        }
        buf.push(b[0]);
    }
    String::from_utf8_lossy(&buf).into_owned()
}
