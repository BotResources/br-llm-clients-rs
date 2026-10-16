use std::sync::Arc;

use rustls::crypto::ring::default_provider;
use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::CertificateDer;
use rustls_pki_types::pem::PemObject;
use tokio_rustls::TlsConnector;

use crate::config_error::ConfigError;

pub(crate) fn build_connector(extra_roots: &[String]) -> Result<TlsConnector, ConfigError> {
    let mut roots = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    for (index, pem) in extra_roots.iter().enumerate() {
        add_pem(&mut roots, pem, index)?;
    }
    let mut config = ClientConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|_| ConfigError::config("the TLS protocol versions are not available"))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(TlsConnector::from(Arc::new(config)))
}

fn add_pem(roots: &mut RootCertStore, pem: &str, index: usize) -> Result<(), ConfigError> {
    let invalid = || {
        ConfigError::config(format!(
            "extra_root_certificates[{index}] is not a valid PEM certificate"
        ))
    };
    let mut found = false;
    for item in CertificateDer::pem_slice_iter(pem.as_bytes()) {
        let der = item.map_err(|_| invalid())?;
        roots.add(der).map_err(|_| invalid())?;
        found = true;
    }
    if found { Ok(()) } else { Err(invalid()) }
}
