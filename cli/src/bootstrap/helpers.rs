use regex::Regex;

/// sn12.23 to SN 12.23
pub fn uid_to_ref(uid: &str) -> String {
    // Add a space after the letters, i.e. the collection abbrev
    let re = Regex::new(r"^([a-z]+)([0-9])").unwrap();
    let mut ref_str = re.replace(uid, "$1 $2").to_string();

    // handle all-upcase collections
    let replacements = [
        ("dn ", "DN "),
        ("mn ", "MN "),
        ("sn ", "SN "),
        ("an ", "AN "),
    ];

    for (from, to) in &replacements {
        ref_str = ref_str.replace(from, to);
    }

    // titlecase the rest, upcase the first letter
    if !ref_str.is_empty() {
        let first_char = ref_str.chars().next().unwrap().to_uppercase().to_string();
        ref_str = first_char + &ref_str[1..];
    }

    ref_str
}

/// sn12.23 to sn
pub fn uid_to_nikaya(uid: &str) -> String {
    let re = Regex::new(r"^([a-z]+).*").unwrap();
    if let Some(caps) = re.captures(uid) {
        caps[1].to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uid_to_ref() {
        assert_eq!(uid_to_ref("dn1"), "DN 1");
        assert_eq!(uid_to_ref("mn2"), "MN 2");
        assert_eq!(uid_to_ref("sn12.23"), "SN 12.23");
        assert_eq!(uid_to_ref("an4.10"), "AN 4.10");
    }

    #[test]
    fn test_uid_to_nikaya() {
        assert_eq!(uid_to_nikaya("dn1"), "dn");
        assert_eq!(uid_to_nikaya("mn2"), "mn");
        assert_eq!(uid_to_nikaya("sn12.23"), "sn");
        assert_eq!(uid_to_nikaya("an4.10"), "an");
    }
}
