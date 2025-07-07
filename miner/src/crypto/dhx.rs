use x25519_dalek::{EphemeralSecret, PublicKey};
use subxt_signer::sr25519::Keypair;

pub struct MinerDH {
    secret: EphemeralSecret,
    pub public: PublicKey,
}

impl MinerDH {
    pub fn new(keypair: &Keypair) -> Self {
        // Convert sr25519 secret key to x25519
        let secret_bytes = keypair.0.secret.to_bytes();
        let secret = EphemeralSecret::from(secret_bytes);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn derive_shared_secret(self, gatekeeper_pub: PublicKey) -> [u8; 32] {
        *self.secret.diffie_hellman(&gatekeeper_pub).as_bytes()
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }
}