/// Represents a specific version of the Minecraft Java Edition protocol.
///
/// Each variant corresponds to a released client version and its associated
/// network protocol number. Ordering reflects chronological release order,
/// allowing version comparisons using standard comparison operators.
///
/// `Unknown` is used when a protocol number is not recognized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[allow(non_camel_case_types)]
pub enum JavaMinecraftVersion {
    /// 1.7.2: The Update That Changed The World.
    V_1_7_2,
    V_1_7_6,
    /// 1.8: The Bountiful Update.
    V_1_8,
    /// 1.9: The Combat Update.
    V_1_9,
    V_1_9_1,
    V_1_9_2,
    V_1_9_3,
    /// 1.10: The Frostburn Update.
    V_1_10,
    /// 1.11: The Exploration Update.
    V_1_11,
    V_1_11_1,
    /// 1.12: The World of Color Update.
    V_1_12,
    V_1_12_1,
    V_1_12_2,
    /// 1.13: Update Aquatic.
    V_1_13,
    V_1_13_1,
    V_1_13_2,
    /// 1.14: Village & Pillage.
    V_1_14,
    V_1_14_1,
    V_1_14_2,
    V_1_14_3,
    V_1_14_4,
    /// 1.15: Buzzy Bees.
    V_1_15,
    V_1_15_1,
    V_1_15_2,
    /// 1.16: Nether Update.
    V_1_16,
    V_1_16_1,
    V_1_16_2,
    V_1_16_3,
    V_1_16_4,
    /// 1.17: Caves & Cliffs: Part I.
    V_1_17,
    V_1_17_1,
    /// 1.18: Caves & Cliffs: Part II.
    V_1_18,
    V_1_18_2,
    /// 1.19: The Wild Update.
    V_1_19,
    V_1_19_1,
    V_1_19_3,
    V_1_19_4,
    /// 1.20: Trails & Tales.
    V_1_20,
    V_1_20_2,
    V_1_20_3,
    /// 1.20.5: Armored Paws.
    V_1_20_5,
    /// 1.21: Tricky Trials.
    V_1_21,
    V_1_21_2,
    V_1_21_4,
    /// 1.21.5: Bundles of Bravery.
    V_1_21_5,
    V_1_21_6,
    V_1_21_7,
    V_1_21_9,
    V_1_21_11,
    //  26.1: Tiny Takeover
    V_26_1,
    V_26_2,
    V_26_3,
    /// Fallback for unrecognized protocol versions.
    Unknown,
}

impl JavaMinecraftVersion {
    /// Returns the network protocol number for this version.
    ///
    /// Returns `-1` for [`JavaMinecraftVersion::Unknown`].
    #[must_use]
    pub const fn protocol_version(&self) -> i32 {
        match self {
            Self::V_1_7_2 => 4,
            Self::V_1_7_6 => 5,
            Self::V_1_8 => 47,
            Self::V_1_9 => 107,
            Self::V_1_9_1 => 108,
            Self::V_1_9_2 => 109,
            Self::V_1_9_3 => 110,
            Self::V_1_10 => 210,
            Self::V_1_11 => 315,
            Self::V_1_11_1 => 316,
            Self::V_1_12 => 335,
            Self::V_1_12_1 => 338,
            Self::V_1_12_2 => 340,
            Self::V_1_13 => 393,
            Self::V_1_13_1 => 401,
            Self::V_1_13_2 => 404,
            Self::V_1_14 => 477,
            Self::V_1_14_1 => 480,
            Self::V_1_14_2 => 485,
            Self::V_1_14_3 => 490,
            Self::V_1_14_4 => 498,
            Self::V_1_15 => 573,
            Self::V_1_15_1 => 575,
            Self::V_1_15_2 => 578,
            Self::V_1_16 => 735,
            Self::V_1_16_1 => 736,
            Self::V_1_16_2 => 751,
            Self::V_1_16_3 => 753,
            Self::V_1_16_4 => 754,
            Self::V_1_17 => 755,
            Self::V_1_17_1 => 756,
            Self::V_1_18 => 757,
            Self::V_1_18_2 => 758,
            Self::V_1_19 => 759,
            Self::V_1_19_1 => 760,
            Self::V_1_19_3 => 761,
            Self::V_1_19_4 => 762,
            Self::V_1_20 => 763,
            Self::V_1_20_2 => 764,
            Self::V_1_20_3 => 765,
            Self::V_1_20_5 => 766,
            Self::V_1_21 => 767,
            Self::V_1_21_2 => 768,
            Self::V_1_21_4 => 769,
            Self::V_1_21_5 => 770,
            Self::V_1_21_6 => 771,
            Self::V_1_21_7 => 772,
            Self::V_1_21_9 => 773,
            Self::V_1_21_11 => 774,
            Self::V_26_1 => 775,
            Self::V_26_2 => 776,
            Self::V_26_3 => 777,
            Self::Unknown => -1,
        }
    }

    /// Resolves a version from a network protocol number.
    ///
    /// Returns [`JavaMinecraftVersion::Unknown`] if the protocol is not supported.
    #[must_use]
    pub const fn from_protocol(protocol: u32) -> Self {
        match protocol {
            4 => Self::V_1_7_2,
            5 => Self::V_1_7_6,
            47 => Self::V_1_8,
            107 => Self::V_1_9,
            108 => Self::V_1_9_1,
            109 => Self::V_1_9_2,
            110 => Self::V_1_9_3,
            210 => Self::V_1_10,
            315 => Self::V_1_11,
            316 => Self::V_1_11_1,
            335 => Self::V_1_12,
            338 => Self::V_1_12_1,
            340 => Self::V_1_12_2,
            393 => Self::V_1_13,
            401 => Self::V_1_13_1,
            404 => Self::V_1_13_2,
            477 => Self::V_1_14,
            480 => Self::V_1_14_1,
            485 => Self::V_1_14_2,
            490 => Self::V_1_14_3,
            498 => Self::V_1_14_4,
            573 => Self::V_1_15,
            575 => Self::V_1_15_1,
            578 => Self::V_1_15_2,
            735 => Self::V_1_16,
            736 => Self::V_1_16_1,
            751 => Self::V_1_16_2,
            753 => Self::V_1_16_3,
            754 => Self::V_1_16_4,
            755 => Self::V_1_17,
            756 => Self::V_1_17_1,
            757 => Self::V_1_18,
            758 => Self::V_1_18_2,
            759 => Self::V_1_19,
            760 => Self::V_1_19_1,
            761 => Self::V_1_19_3,
            762 => Self::V_1_19_4,
            763 => Self::V_1_20,
            764 => Self::V_1_20_2,
            765 => Self::V_1_20_3,
            766 => Self::V_1_20_5,
            767 => Self::V_1_21,
            768 => Self::V_1_21_2,
            769 => Self::V_1_21_4,
            770 => Self::V_1_21_5,
            771 => Self::V_1_21_6,
            772 => Self::V_1_21_7,
            773 => Self::V_1_21_9,
            774 => Self::V_1_21_11,
            775 => Self::V_26_1,
            776 => Self::V_26_2,
            777 => Self::V_26_3,
            _ => Self::Unknown,
        }
    }

    #[inline]
    #[must_use]
    pub const fn supports_configuration_state(&self) -> bool {
        self.protocol_version() >= Self::V_1_20_2.protocol_version()
    }

    #[inline]
    #[must_use]
    pub const fn is_modern(&self) -> bool {
        self.protocol_version() >= Self::V_1_13.protocol_version()
    }

    #[inline]
    #[must_use]
    pub const fn has_registries(&self) -> bool {
        self.protocol_version() >= Self::V_1_16.protocol_version()
    }

    /// Returns `true` for versions that use the structured Data Components item format
    /// introduced in 1.20.5 (protocol 766), replacing legacy NBT item stacks.
    #[inline]
    #[must_use]
    pub const fn has_data_components(&self) -> bool {
        self.protocol_version() >= Self::V_1_20_5.protocol_version()
    }

    /// Returns `true` for versions that support the Known Packs negotiation during
    /// the Configuration state (added in 1.20.5, protocol 766).
    #[inline]
    #[must_use]
    pub const fn supports_known_packs(&self) -> bool {
        self.protocol_version() >= Self::V_1_20_5.protocol_version()
    }

    /// Returns `true` for versions that include the sea level field in the Login packet
    /// (added in 1.21.2, protocol 768).
    #[inline]
    #[must_use]
    pub const fn has_sea_level_in_login(&self) -> bool {
        self.protocol_version() >= Self::V_1_21_2.protocol_version()
    }

    /// Returns `true` for versions that include the portal cooldown field in the Login packet
    /// (added in 1.20, protocol 763).
    #[inline]
    #[must_use]
    pub const fn has_portal_cooldown_in_login(&self) -> bool {
        self.protocol_version() >= Self::V_1_20.protocol_version()
    }

    /// Returns `true` for versions that include the `liquid_count` field in chunk section headers
    /// (added in 26.1, protocol 775).
    #[inline]
    #[must_use]
    pub const fn has_chunk_liquid_count(&self) -> bool {
        self.protocol_version() >= Self::V_26_1.protocol_version()
    }

    /// Returns `true` for versions that use a compact indexed heightmap format (added in 1.21.5,
    /// protocol 770) instead of NBT-encoded heightmaps.
    #[inline]
    #[must_use]
    pub const fn has_indexed_heightmaps(&self) -> bool {
        self.protocol_version() >= Self::V_1_21_5.protocol_version()
    }

    /// Returns `true` if the client is within the supported multi-version range
    /// (1.21 through 26.3, protocols 767–777).
    #[inline]
    #[must_use]
    pub const fn is_in_supported_range(&self) -> bool {
        let proto = self.protocol_version();
        proto >= Self::V_1_21.protocol_version() && proto <= Self::V_26_3.protocol_version()
    }
}

impl std::fmt::Display for JavaMinecraftVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V_1_7_2 => write!(f, "1.7.2"),
            Self::V_1_7_6 => write!(f, "1.7.6"),
            Self::V_1_8 => write!(f, "1.8"),
            Self::V_1_9 => write!(f, "1.9"),
            Self::V_1_9_1 => write!(f, "1.9.1"),
            Self::V_1_9_2 => write!(f, "1.9.2"),
            Self::V_1_9_3 => write!(f, "1.9.3"),
            Self::V_1_10 => write!(f, "1.10"),
            Self::V_1_11 => write!(f, "1.11"),
            Self::V_1_11_1 => write!(f, "1.11.1"),
            Self::V_1_12 => write!(f, "1.12"),
            Self::V_1_12_1 => write!(f, "1.12.1"),
            Self::V_1_12_2 => write!(f, "1.12.2"),
            Self::V_1_13 => write!(f, "1.13"),
            Self::V_1_13_1 => write!(f, "1.13.1"),
            Self::V_1_13_2 => write!(f, "1.13.2"),
            Self::V_1_14 => write!(f, "1.14"),
            Self::V_1_14_1 => write!(f, "1.14.1"),
            Self::V_1_14_2 => write!(f, "1.14.2"),
            Self::V_1_14_3 => write!(f, "1.14.3"),
            Self::V_1_14_4 => write!(f, "1.14.4"),
            Self::V_1_15 => write!(f, "1.15"),
            Self::V_1_15_1 => write!(f, "1.15.1"),
            Self::V_1_15_2 => write!(f, "1.15.2"),
            Self::V_1_16 => write!(f, "1.16"),
            Self::V_1_16_1 => write!(f, "1.16.1"),
            Self::V_1_16_2 => write!(f, "1.16.2"),
            Self::V_1_16_3 => write!(f, "1.16.3"),
            Self::V_1_16_4 => write!(f, "1.16.4"),
            Self::V_1_17 => write!(f, "1.17"),
            Self::V_1_17_1 => write!(f, "1.17.1"),
            Self::V_1_18 => write!(f, "1.18"),
            Self::V_1_18_2 => write!(f, "1.18.2"),
            Self::V_1_19 => write!(f, "1.19"),
            Self::V_1_19_1 => write!(f, "1.19.1"),
            Self::V_1_19_3 => write!(f, "1.19.3"),
            Self::V_1_19_4 => write!(f, "1.19.4"),
            Self::V_1_20 => write!(f, "1.20"),
            Self::V_1_20_2 => write!(f, "1.20.2"),
            Self::V_1_20_3 => write!(f, "1.20.3"),
            Self::V_1_20_5 => write!(f, "1.20.5"),
            Self::V_1_21 => write!(f, "1.21"),
            Self::V_1_21_2 => write!(f, "1.21.2"),
            Self::V_1_21_4 => write!(f, "1.21.4"),
            Self::V_1_21_5 => write!(f, "1.21.5"),
            Self::V_1_21_6 => write!(f, "1.21.6"),
            Self::V_1_21_7 => write!(f, "1.21.7"),
            Self::V_1_21_9 => write!(f, "1.21.9"),
            Self::V_1_21_11 => write!(f, "1.21.11"),
            Self::V_26_1 => write!(f, "26.1"),
            Self::V_26_2 => write!(f, "26.2"),
            Self::V_26_3 => write!(f, "26.3"),

            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Represents a specific version of the Minecraft Bedrock Edition protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[allow(non_camel_case_types)]
pub enum BedrockMinecraftVersion {
    /// 1.21: Tricky Trials.
    V_1_21,
    /// 1.26.45
    V_1_26_45,
    /// Fallback for unrecognized protocol versions.
    Unknown,
}

impl BedrockMinecraftVersion {
    /// Returns the network protocol number for this version.
    ///
    /// Returns `-1` for [`BedrockMinecraftVersion::Unknown`].
    #[must_use]
    pub const fn protocol_version(&self) -> i32 {
        match self {
            Self::V_1_21 => 671,
            Self::V_1_26_45 => 2169,
            Self::Unknown => -1,
        }
    }

    /// Resolves a version from a network protocol number.
    ///
    /// Returns [`BedrockMinecraftVersion::Unknown`] if the protocol is not supported.
    #[must_use]
    pub const fn from_protocol(protocol: u32) -> Self {
        match protocol {
            671 => Self::V_1_21,
            2169 => Self::V_1_26_45,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for BedrockMinecraftVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V_1_21 => write!(f, "1.21"),
            Self::V_1_26_45 => write!(f, "1.26.45"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BedrockMinecraftVersion, JavaMinecraftVersion};

    #[test]
    fn resolves_bedrock_26_45_protocol() {
        let version = BedrockMinecraftVersion::from_protocol(2169);

        assert_eq!(version, BedrockMinecraftVersion::V_1_26_45);
        assert_eq!(version.protocol_version(), 2169);
        assert_eq!(version.to_string(), "1.26.45");
    }

    #[test]
    fn has_data_components_gating() {
        // 1.20.5 and later have data components
        assert!(JavaMinecraftVersion::V_1_20_5.has_data_components());
        assert!(JavaMinecraftVersion::V_1_21.has_data_components());
        assert!(JavaMinecraftVersion::V_1_21_2.has_data_components());
        assert!(JavaMinecraftVersion::V_1_21_4.has_data_components());
        assert!(JavaMinecraftVersion::V_26_1.has_data_components());
        assert!(JavaMinecraftVersion::V_26_2.has_data_components());

        // Older versions do not
        assert!(!JavaMinecraftVersion::V_1_20_3.has_data_components());
        assert!(!JavaMinecraftVersion::V_1_20_2.has_data_components());
        assert!(!JavaMinecraftVersion::V_1_20.has_data_components());
    }

    #[test]
    fn has_chunk_liquid_count_gating() {
        assert!(JavaMinecraftVersion::V_26_1.has_chunk_liquid_count());
        assert!(JavaMinecraftVersion::V_26_2.has_chunk_liquid_count());
        // Versions below 26.1 must not include liquid_count
        assert!(!JavaMinecraftVersion::V_1_21.has_chunk_liquid_count());
        assert!(!JavaMinecraftVersion::V_1_21_2.has_chunk_liquid_count());
        assert!(!JavaMinecraftVersion::V_1_21_4.has_chunk_liquid_count());
        assert!(!JavaMinecraftVersion::V_1_21_5.has_chunk_liquid_count());
    }

    #[test]
    fn has_indexed_heightmaps_gating() {
        assert!(JavaMinecraftVersion::V_1_21_5.has_indexed_heightmaps());
        assert!(JavaMinecraftVersion::V_26_1.has_indexed_heightmaps());
        assert!(JavaMinecraftVersion::V_26_2.has_indexed_heightmaps());
        // Versions below 1.21.5 use NBT heightmaps
        assert!(!JavaMinecraftVersion::V_1_21.has_indexed_heightmaps());
        assert!(!JavaMinecraftVersion::V_1_21_2.has_indexed_heightmaps());
        assert!(!JavaMinecraftVersion::V_1_21_4.has_indexed_heightmaps());
    }

    #[test]
    fn has_sea_level_in_login_gating() {
        assert!(JavaMinecraftVersion::V_1_21_2.has_sea_level_in_login());
        assert!(JavaMinecraftVersion::V_1_21_4.has_sea_level_in_login());
        assert!(JavaMinecraftVersion::V_26_1.has_sea_level_in_login());
        assert!(JavaMinecraftVersion::V_26_2.has_sea_level_in_login());
        // 1.21 does not have sea level
        assert!(!JavaMinecraftVersion::V_1_21.has_sea_level_in_login());
    }

    #[test]
    fn is_in_supported_range_checks() {
        // All target versions are in range
        for proto in 767u32..=777 {
            let version = JavaMinecraftVersion::from_protocol(proto);
            assert!(
                version.is_in_supported_range(),
                "Protocol {proto} ({version}) should be in supported range"
            );
        }
        // Legacy versions are not
        assert!(!JavaMinecraftVersion::V_1_20_5.is_in_supported_range());
        assert!(!JavaMinecraftVersion::V_1_20_3.is_in_supported_range());
        assert!(!JavaMinecraftVersion::Unknown.is_in_supported_range());
    }

    #[test]
    fn supports_configuration_state_covers_1_21_range() {
        // All 1.21+ clients support config state
        assert!(JavaMinecraftVersion::V_1_21.supports_configuration_state());
        assert!(JavaMinecraftVersion::V_1_21_2.supports_configuration_state());
        assert!(JavaMinecraftVersion::V_1_21_4.supports_configuration_state());
        assert!(JavaMinecraftVersion::V_26_1.supports_configuration_state());
        assert!(JavaMinecraftVersion::V_26_2.supports_configuration_state());
        assert!(JavaMinecraftVersion::V_26_3.supports_configuration_state());
    }

    #[test]
    fn protocol_version_roundtrips_for_all_target_versions() {
        let target_protocols = [767u32, 768, 769, 770, 771, 772, 773, 774, 775, 776, 777];
        for proto in target_protocols {
            let version = JavaMinecraftVersion::from_protocol(proto);
            assert_ne!(
                version,
                JavaMinecraftVersion::Unknown,
                "Protocol {proto} must resolve to a known version"
            );
            assert_eq!(
                version.protocol_version() as u32,
                proto,
                "Protocol {proto} round-trip mismatch"
            );
        }
    }
}
