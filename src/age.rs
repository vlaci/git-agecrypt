use std::{
    io::{self, Read},
    path::Path,
};

use age::{
    armor::ArmoredReader,
    cli_common::{read_identities, UiCallbacks},
    plugin::{self, RecipientPluginV1},
    DecryptError, Decryptor, Encryptor, Identity, Recipient,
};
use anyhow::{bail, Result};

pub(crate) fn decrypt(
    identities: &[impl AsRef<Path>],
    encrypted: &mut impl Read,
) -> Result<Option<Vec<u8>>> {
    let id = read_identities(
        identities
            .iter()
            .map(|i| i.as_ref().to_string_lossy().into())
            .collect(),
        None,
    )?;
    let id = id.iter().map(|i| i.as_ref() as &dyn Identity);
    let mut decrypted = vec![];
    let decryptor = match Decryptor::new(ArmoredReader::new(encrypted)) {
        Ok(Decryptor::Recipients(d)) => d,
        Ok(Decryptor::Passphrase(_)) => bail!("Passphrase encrypted files are not supported"),
        Err(DecryptError::InvalidHeader) => return Ok(None),
        Err(e) => bail!(e),
    };

    let mut reader = decryptor.decrypt(id)?;
    reader.read_to_end(&mut decrypted)?;
    Ok(Some(decrypted))
}

pub(crate) fn encrypt(
    public_keys: &[impl AsRef<str>],
    cleartext: &mut impl Read,
) -> Result<Vec<u8>> {
    let mut recipients: Vec<Box<dyn Recipient>> = vec![];
    let mut plugin_recipients = vec![];
    let callbacks = UiCallbacks {};

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

    for plugin_name in plugin_recipients.iter().map(|r| r.plugin()) {
        let recipient = RecipientPluginV1::new(plugin_name, &plugin_recipients, &[], callbacks)?;
        recipients.push(Box::new(recipient));
    }

    let encryptor = Encryptor::with_recipients(recipients);
    let mut encrypted = vec![];

    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    io::copy(cleartext, &mut writer)?;
    writer.finish()?;
    Ok(encrypted)
}
