//! School-server TLS certificate + pinned client verifier (prompts/P04 Step 2,
//! docs §9). Self-signed `rcgen` cert (10 years), created at setup, key stored in
//! the encrypted DB. Fingerprint = SHA-256 of the DER cert. Clients pin ONLY that
//! fingerprint (no public CA) — a different cert is rejected.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, ServerConfig, SignatureScheme};
use sha2::{Digest, Sha256};

/// The server's cert material (persisted in the encrypted `school` settings/DB).
#[derive(Debug, Clone)]
pub struct CertMaterial {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
    /// SHA-256 of the DER cert, lower-case hex.
    pub fingerprint: String,
}

/// SHA-256 of DER bytes as lower-case hex (the pinned fingerprint).
pub fn fingerprint_of_der(der: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(der);
    let out = h.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Generate a fresh self-signed cert valid for 10 years.
pub fn generate_school_cert() -> Result<CertMaterial, String> {
    let key_pair = rcgen::KeyPair::generate().map_err(|e| e.to_string())?;
    let mut params = rcgen::CertificateParams::new(vec!["vidya-school".to_string()]).map_err(|e| e.to_string())?;
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now - time::Duration::days(1);
    params.not_after = now + time::Duration::days(3652); // ~10 years
    let cert = params.self_signed(&key_pair).map_err(|e| e.to_string())?;
    let cert_der = cert.der().as_ref().to_vec();
    let fingerprint = fingerprint_of_der(&cert_der);
    Ok(CertMaterial { cert_der, key_der: key_pair.serialize_der(), fingerprint })
}

/// A rustls verifier that accepts ONLY the pinned SHA-256 fingerprint, but still
/// verifies the handshake signature (so a MITM can't present the pinned cert
/// without holding its key). Name and expiry are intentionally ignored.
#[derive(Debug)]
pub struct PinnedServerCertVerifier {
    fingerprint: String,
    schemes: WebPkiSupportedAlgorithms,
}

impl PinnedServerCertVerifier {
    pub fn new(fingerprint: impl Into<String>) -> Self {
        let provider = rustls::crypto::ring::default_provider();
        Self { fingerprint: fingerprint.into(), schemes: provider.signature_verification_algorithms }
    }
}

impl ServerCertVerifier for PinnedServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if fingerprint_of_der(end_entity.as_ref()) == self.fingerprint {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.schemes)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.schemes)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes.supported_schemes()
    }
}

/// A client TLS config that trusts ONLY the pinned fingerprint (ring provider).
pub fn client_config(fingerprint: impl Into<String>) -> Result<Arc<ClientConfig>, String> {
    let cfg = ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinnedServerCertVerifier::new(fingerprint)))
        .with_no_client_auth();
    Ok(Arc::new(cfg))
}

/// The server TLS config from stored DER cert + key (ring provider).
pub fn server_config(cert_der: &[u8], key_der: &[u8]) -> Result<Arc<ServerConfig>, String> {
    let certs = vec![CertificateDer::from(cert_der.to_vec())];
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der.to_vec()));
    let cfg = ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(cfg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::pki_types::{ServerName, UnixTime};

    #[test]
    fn fingerprint_is_stable_sha256_hex() {
        let fp = fingerprint_of_der(b"hello");
        // Known SHA-256("hello").
        assert_eq!(fp, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        assert_eq!(fp.len(), 64);
    }

    #[test]
    fn generated_cert_fingerprint_matches_der() {
        let m = generate_school_cert().unwrap();
        assert_eq!(m.fingerprint, fingerprint_of_der(&m.cert_der));
        assert!(!m.key_der.is_empty());
    }

    #[test]
    fn pinned_verifier_accepts_only_the_pinned_cert() {
        let server = generate_school_cert().unwrap();
        let other = generate_school_cert().unwrap();
        let verifier = PinnedServerCertVerifier::new(server.fingerprint.clone());
        let name = ServerName::try_from("vidya-school").unwrap();
        let now = UnixTime::since_unix_epoch(std::time::Duration::from_secs(1_800_000_000));

        // The pinned cert is accepted.
        assert!(verifier
            .verify_server_cert(&CertificateDer::from(server.cert_der.clone()), &[], &name, &[], now)
            .is_ok());
        // A DIFFERENT self-signed cert is rejected (docs §9 / DONE-MEANS security).
        assert!(verifier
            .verify_server_cert(&CertificateDer::from(other.cert_der.clone()), &[], &name, &[], now)
            .is_err());
    }

    #[test]
    fn configs_build() {
        let m = generate_school_cert().unwrap();
        assert!(client_config(m.fingerprint.clone()).is_ok());
        assert!(server_config(&m.cert_der, &m.key_der).is_ok());
    }
}
