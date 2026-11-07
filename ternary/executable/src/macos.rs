//! macOS bundle and dynamic-link boundary. Filesystem access is supplied by a broker.

use crate::{select_app_executable, Error, MAX_IMAGE_SIZE};
use std::path::{Component, Path};

pub const MAX_BUNDLE_MEMBERS: usize = 65_536;
pub const MAX_PLIST_SIZE: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleMember { pub relative_path:String, pub size:u64, pub is_directory:bool }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundlePlan { pub executable:String, pub resources:Vec<String> }

pub fn validate_member_path(path:&str)->Result<(),Error>{
    if path.is_empty() || path.len()>4096 || path.contains('\0') { return Err(Error::Invalid("bundle member path")); }
    let p=Path::new(path);
    if p.is_absolute() || p.components().any(|c|matches!(c,Component::ParentDir|Component::RootDir|Component::Prefix(_))) {
        return Err(Error::Invalid("bundle path traversal"));
    }
    Ok(())
}

/// Build a deterministic bundle plan from broker-enumerated members and XML Info.plist.
/// Symlinks are not accepted by this interface; the filesystem broker must reject them.
pub fn plan_bundle(plist:&[u8], members:&[BundleMember])->Result<BundlePlan,Error>{
    if plist.len()>MAX_PLIST_SIZE || members.len()>MAX_BUNDLE_MEMBERS { return Err(Error::Limit("bundle metadata")); }
    let mut names=Vec::new();
    for m in members {
        validate_member_path(&m.relative_path)?;
        if m.size>MAX_IMAGE_SIZE as u64 { return Err(Error::Limit("bundle member size")); }
        if !m.is_directory { names.push(m.relative_path.as_str()); }
    }
    let exe=select_app_executable(plist,&names)?;
    let mut resources=Vec::new();
    for m in members {
        if !m.is_directory && m.relative_path!=exe && (m.relative_path.starts_with("Resources/") || m.relative_path=="Resources") {
            resources.push(m.relative_path.clone());
        }
    }
    resources.sort();
    Ok(BundlePlan{executable:exe.to_owned(),resources})
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameworkDecision { pub install_name:String, pub resolved:bool, pub diagnostic:&'static str }
pub fn unresolved_framework(name:&str)->Result<FrameworkDecision,Error>{
    if name.is_empty()||name.len()>4096||name.contains('\0') { return Err(Error::Invalid("Mach-O install name")); }
    Ok(FrameworkDecision{install_name:name.to_owned(),resolved:false,diagnostic:"framework resolution requires a brokered, versioned framework catalog"})
}
