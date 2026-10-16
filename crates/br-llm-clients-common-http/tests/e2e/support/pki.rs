use std::net::Ipv4Addr;
use std::sync::{Arc, OnceLock};

use rcgen::{
    BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose, SanType,
};
use rustls::ServerConfig;
use rustls::crypto::ring::default_provider;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::TlsAcceptor;

pub struct Pki {
    pub ca_pem: String,
    leaf: CertificateDer<'static>,
    key: Vec<u8>,
}

impl Pki {
    fn generate(names: &[&str], ip: bool) -> Self {
        let ca_key = KeyPair::generate().expect("ca key");
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).expect("ca params");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        ca_params
            .distinguished_name
            .push(DnType::CommonName, "br-llm-clients test ca");
        let ca = ca_params.self_signed(&ca_key).expect("ca");
        let issuer = Issuer::new(ca_params, ca_key);
        let leaf_key = KeyPair::generate().expect("leaf key");
        let mut leaf_params =
            CertificateParams::new(names.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>())
                .expect("leaf params");
        if ip {
            leaf_params
                .subject_alt_names
                .push(SanType::IpAddress(Ipv4Addr::LOCALHOST.into()));
        }
        let leaf = leaf_params.signed_by(&leaf_key, &issuer).expect("leaf");
        Self {
            ca_pem: ca.pem(),
            leaf: leaf.der().clone(),
            key: leaf_key.serialize_der(),
        }
    }

    pub fn trusted() -> &'static Pki {
        static CELL: OnceLock<Pki> = OnceLock::new();
        CELL.get_or_init(|| Pki::generate(&["localhost"], true))
    }

    pub fn other_ca() -> &'static Pki {
        static CELL: OnceLock<Pki> = OnceLock::new();
        CELL.get_or_init(|| Pki::generate(&["localhost"], true))
    }

    pub fn wrong_name() -> &'static Pki {
        static CELL: OnceLock<Pki> = OnceLock::new();
        CELL.get_or_init(|| Pki::generate(&["other.example"], false))
    }

    pub fn acceptor(&self, alpn: &[&[u8]]) -> TlsAcceptor {
        let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(self.key.clone()));
        let mut config = ServerConfig::builder_with_provider(Arc::new(default_provider()))
            .with_safe_default_protocol_versions()
            .expect("versions")
            .with_no_client_auth()
            .with_single_cert(vec![self.leaf.clone()], key)
            .expect("server config");
        config.alpn_protocols = alpn.iter().map(|p| p.to_vec()).collect();
        TlsAcceptor::from(Arc::new(config))
    }
}
