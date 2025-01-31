// SPDX-License-Identifier: Apache-2.0
// This file contains code for fetching derived keys from root keys. It also includes functions for requesting and saving derived keys.

use super::*;
use sev::firmware::guest::{DerivedKey, Firmware, GuestFieldSelect};
use std::io::Read;
use std::{fs, path::PathBuf};

#[derive(Parser)]
pub struct KeyArgs {
    /// This is the path where the derived key will be saved.
    #[arg(value_name = "key-path", required = true)]
    pub key_path: PathBuf,

    /// This is the root key from which to derive the key. Input either VCEK or VMRK.
    #[arg(value_name = "root-key-select", required = true, ignore_case = true)]
    pub root_key_select: String,

    /// Specify an integer VMPL level between 0 and 3 that the Guest is running on.
    #[arg(short, long, value_name = "vmpl", default_value = "1")]
    pub vmpl: Option<u32>,

    /// Specify which Guest Field Select bits to enable. It is a 6 digit binary string. For each bit, 0 denotes off and 1 denotes on.
    /// The least significant (rightmost) bit is Guest Policy followed by Image ID, Family ID, Measurement, SVN, TCB Version which is the most significant (leftmost) bit.
    #[arg(short, long = "guest_field_select", value_name = "######")]
    pub gfs: Option<String>,

    /// Specify the guest SVN to mix into the key. Must not exceed the guest SVN provided at launch in the ID block.
    #[arg(short = 's', long = "guest_svn")]
    pub gsvn: Option<u32>,

    /// Specify the TCB version to mix into the derived key. Must not exceed CommittedTcb.
    #[arg(short, long = "tcb_version")]
    pub tcbv: Option<u64>,
}

pub fn get_derived_key(args: KeyArgs) -> Result<()> {
    let root_key_select = match args.root_key_select.as_str() {
        "vcek" => false,
        "vmrk" => true,
        _ => return Err(anyhow::anyhow!("Invalid input. Enter either vcek or vmrk")),
    };

    let vmpl = match args.vmpl {
        Some(level) => {
            if level <= 3 {
                level
            } else {
                return Err(anyhow::anyhow!("Invalid Virtual Machine Privilege Level."));
            }
        }
        None => 1,
    };

    let gfs = match args.gfs {
        Some(gfs) => {
            let value: u64 = u64::from_str_radix(gfs.as_str(), 2).unwrap();
            if value <= 63 {
                value
            } else {
                return Err(anyhow::anyhow!("Invalid Guest Field Select option."));
            }
        }
        None => 0,
    };

    let gsvn: u32 = args.gsvn.unwrap_or(0);

    let tcbv: u64 = args.tcbv.unwrap_or(0);

    let request = DerivedKey::new(root_key_select, GuestFieldSelect(gfs), vmpl, gsvn, tcbv);
    let mut sev_fw = Firmware::open().context("failed to open SEV firmware device.")?;
    let derived_key: [u8; 32] = sev_fw
        .get_derived_key(None, request)
        .context("Failed to request derived key")?;

    // Create derived key path
    let key_path: PathBuf = args.key_path;

    // Write derived key into desired file
    let mut key_file = if key_path.exists() {
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(key_path)
            .context("Unable to overwrite derived key file contents")?
    } else {
        fs::File::create(key_path).context("Unable to create derived key file contents")?
    };

    bincode::serialize_into(&mut key_file, &derived_key)
        .context("Could not serialize derived key into file.")?;

    Ok(())
}

pub fn read_key(key_path: PathBuf) -> Result<Vec<u8>, anyhow::Error> {
    let mut key_file = fs::File::open(key_path)?;
    let mut key = Vec::new();
    // read the whole file
    key_file.read_to_end(&mut key)?;
    Ok(key)
}
