//! Broker-facing application dispatch for original executable files and .app bundles.
//! The dispatcher never extracts, rewrites, translates, or executes bundle contents.

use crate::{Error, engine::LoadPlan};
use crate::macos::BundleMember;

pub const MAX_ARGUMENTS: usize = 4096;
pub const MAX_ARGUMENT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug)]
pub enum ApplicationInput {
    PeExecutable { original_path: String, bytes: Vec<u8> },
    MachOExecutable { original_path: String, bytes: Vec<u8> },
    MacAppBundle {
        original_path: String,
        info_plist: Vec<u8>,
        members: Vec<BundleMember>,
        executable_bytes: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub format: crate::Format,
    pub architecture: crate::Architecture,
    pub image_prepared: bool,
    pub guest_backend_integrated: bool,
    pub can_launch: bool,
    pub blockers: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub struct ApplicationLaunchPlan {
    /// User-selected original path; it is not replaced by an extracted copy.
    pub original_path: String,
    /// Relative executable member for .app bundles; None for standalone files.
    pub executable_member: Option<String>,
    pub load_plan: LoadPlan,
    pub compatibility: CompatibilityReport,
}

fn validate_original_path(path: &str) -> Result<(), Error> {
    if path.is_empty() || path.len() > 4096 || path.contains('\0') {
        return Err(Error::Invalid("application path"));
    }
    Ok(())
}

fn validate_arguments(args: &[String], env: &[String]) -> Result<(), Error> {
    if args.len().checked_add(env.len()).ok_or(Error::Overflow)? > MAX_ARGUMENTS {
        return Err(Error::Limit("application argument/environment count"));
    }
    let total = args.iter().chain(env).try_fold(0usize, |n, item| {
        if item.contains('\0') { return Err(Error::Invalid("NUL in application argument")); }
        n.checked_add(item.len().checked_add(1).ok_or(Error::Overflow)?).ok_or(Error::Overflow)
    })?;
    if total > MAX_ARGUMENT_BYTES { return Err(Error::Limit("application argument/environment bytes")); }
    Ok(())
}

/// Prepare the original application through one format-neutral dispatch path.
/// This is a checked plan, not a claim that the kernel can enter the guest yet.
pub fn prepare_application(
    input: ApplicationInput,
    arguments: &[String],
    environment: &[String],
) -> Result<ApplicationLaunchPlan, Error> {
    validate_arguments(arguments, environment)?;
    let (original_path, executable_member, load_plan) = match input {
        ApplicationInput::PeExecutable { original_path, bytes } => {
            validate_original_path(&original_path)?;
            let plan = crate::engine::prepare_pe32_plus(&bytes, None, None)?;
            (original_path, None, plan)
        }
        ApplicationInput::MachOExecutable { original_path, bytes } => {
            validate_original_path(&original_path)?;
            let plan = crate::engine::prepare_macho64(&bytes, None)?;
            (original_path, None, plan)
        }
        ApplicationInput::MacAppBundle {
            original_path,
            info_plist,
            members,
            executable_bytes,
        } => {
            validate_original_path(&original_path)?;
            let bundle = crate::macos::plan_bundle(&info_plist, &members)?;
            let member = members.iter().find(|m| m.relative_path == bundle.executable)
                .ok_or(Error::Invalid("selected app executable missing from broker manifest"))?;
            if member.is_directory || member.size != executable_bytes.len() as u64 {
                return Err(Error::Invalid("selected app executable size/type mismatch"));
            }
            let plan = crate::engine::prepare_macho64(&executable_bytes, None)?;
            (original_path, Some(bundle.executable), plan)
        }
    };
    let compatibility = CompatibilityReport {
        format: load_plan.format,
        architecture: load_plan.architecture,
        image_prepared: true,
        guest_backend_integrated: false,
        can_launch: false,
        blockers: vec![
            "kernel guest-process creation and native execution boundary are not integrated",
            "platform syscall/API service broker is not integrated",
            "instruction and process-isolation validation is deferred until Phase 2 implementation is complete",
        ],
    };
    Ok(ApplicationLaunchPlan {
        original_path,
        executable_member,
        load_plan,
        compatibility,
    })
}
