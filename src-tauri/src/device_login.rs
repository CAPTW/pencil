pub(crate) const PINNED_DEVICE_LOGIN_URL: &str = "https://auth.openai.com/codex/device";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ValidatedDeviceLoginUrl(String);

impl ValidatedDeviceLoginUrl {
    pub(crate) fn parse(value: &str) -> Result<Self, &'static str> {
        // The pinned managed login authority has one exact stable URL. Exact
        // ASCII equality rejects alternate schemes, ports, credentials,
        // fragments, IP literals, Unicode/percent-encoding, and host/path
        // confusion without accepting a broader URL surface.
        if value == PINNED_DEVICE_LOGIN_URL {
            Ok(Self(value.to_string()))
        } else {
            Err("device_url_authority_mismatch")
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_pinned_device_url_is_the_only_accepted_authority() {
        let accepted = ValidatedDeviceLoginUrl::parse(PINNED_DEVICE_LOGIN_URL)
            .expect("the pinned managed-login URL must be accepted");
        assert_eq!(accepted.as_str(), PINNED_DEVICE_LOGIN_URL);

        let rejected = [
            "http://auth.openai.com/codex/device",
            "file:///codex/device",
            "javascript:alert(1)",
            "data:text/plain,device",
            "https://localhost/codex/device",
            "https://127.0.0.1/codex/device",
            "https://sub.auth.openai.com/codex/device",
            "https://auth.openai.com.evil.example/codex/device",
            "https://auth.openai.com:444/codex/device",
            "https://user@auth.openai.com/codex/device",
            "https://auth.openai.com/wrong",
            "https://auth.openai.com/codex/device#fragment",
            "https://auth%2eopenai.com/codex/device",
            "https://auth.openai.com/codex/%64evice",
            "https://AUTH.OPENAI.COM/codex/device",
            " https://auth.openai.com/codex/device",
            "https://auth.openai.com/codex/device?next=other",
        ];

        for candidate in rejected {
            assert_eq!(
                ValidatedDeviceLoginUrl::parse(candidate),
                Err("device_url_authority_mismatch")
            );
        }
    }
}
