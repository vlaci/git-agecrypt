use std::{
    io::{self, ErrorKind as IoErrorKind, Read},
    path::Path,
};

use age::{
    armor::ArmoredReader,
    cli_common::{read_identities, StdinGuard, UiCallbacks},
    plugin::{self, RecipientPluginV1},
    DecryptError, Decryptor, Encryptor, Identity, Recipient,
};
use anyhow::{bail, Context, Result};

pub(crate) fn decrypt(
    identities: &[impl AsRef<Path>],
    encrypted: &mut impl Read,
) -> Result<Option<Vec<u8>>> {
    let id = load_identities(identities)?;
    let id = id.iter().map(|i| i.as_ref() as &dyn Identity);
    let mut decrypted = vec![];
    let decryptor = match Decryptor::new(ArmoredReader::new(encrypted)) {
        Ok(Decryptor::Recipients(d)) => d,
        Ok(Decryptor::Passphrase(_)) => bail!("Passphrase encrypted files are not supported"),
        Err(DecryptError::InvalidHeader) => return Ok(None),
        Err(DecryptError::Io(e)) => {
            match e.kind() {
                // Age gives unexpected EOF when the file contains not enough data
                IoErrorKind::UnexpectedEof => return Ok(None),
                _ => bail!(e),
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
            bail!(e)
        }
    };

    let mut reader = decryptor.decrypt(id)?;
    reader.read_to_end(&mut decrypted)?;
    Ok(Some(decrypted))
}

fn load_identities(identities: &[impl AsRef<Path>]) -> Result<Vec<Box<dyn Identity>>> {
    let id: Vec<String> = identities
        .iter()
        .map(|i| i.as_ref().to_string_lossy().into())
        .collect();
    let mut stdin_guard = StdinGuard::new(false);
    let rv = read_identities(id.clone(), None, &mut stdin_guard)
        .with_context(|| format!("Loading identities failed from paths: {:?}", id))?;
    Ok(rv)
}

pub(crate) fn encrypt(
    public_keys: &[impl AsRef<str> + std::fmt::Debug],
    cleartext: &mut impl Read,
) -> Result<Vec<u8>> {
    let recipients = load_public_keys(public_keys)?;

    let encryptor = Encryptor::with_recipients(recipients).with_context(|| {
        format!(
            "Couldn't load keys for recepients; public_keys={:?}",
            public_keys
        )
    })?;
    let mut encrypted = vec![];

    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    io::copy(cleartext, &mut writer)?;
    writer.finish()?;
    Ok(encrypted)
}

fn load_public_keys(public_keys: &[impl AsRef<str>]) -> Result<Vec<Box<dyn Recipient + Send>>> {
    let mut recipients: Vec<Box<dyn Recipient + Send>> = vec![];
    let mut plugin_recipients = vec![];

    for pubk in public_keys {
        if let Ok(pk) = pubk.as_ref().parse::<age::x25519::Recipient>() {
            recipients.push(Box::new(pk));
        } else if let Ok(pk) = pubk.as_ref().parse::<age::ssh::Recipient>() {
            recipients.push(Box::new(pk));
        } else if let Ok(recipient) = pubk.as_ref().parse::<plugin::Recipient>() {
            plugin_recipients.push(recipient);
        } else {
            bail!("Invalid recipient");
        }
    }
    let callbacks = UiCallbacks {};

    for plugin_name in plugin_recipients.iter().map(|r| r.plugin()) {
        let recipient = RecipientPluginV1::new(plugin_name, &plugin_recipients, &[], callbacks)?;
        recipients.push(Box::new(recipient));
    }

    Ok(recipients)
}

pub(crate) fn validate_public_keys(public_keys: &[impl AsRef<str>]) -> Result<()> {
    load_public_keys(public_keys)?;
    Ok(())
}

pub(crate) fn validate_identity(identity: impl AsRef<Path>) -> Result<()> {
    let mut stdin_guard = StdinGuard::new(false);
    read_identities(
        vec![identity.as_ref().to_string_lossy().into()],
        None,
        &mut stdin_guard,
    )?;
    Ok(())
}
