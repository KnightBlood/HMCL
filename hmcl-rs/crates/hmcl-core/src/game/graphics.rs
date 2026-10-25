//! The graphics interfaces the game can draw with.
//!
//! The game draws its picture with OpenGL or, since a few versions, with
//! Vulkan. The launcher chooses the interface and, on some systems, the driver
//! the game loads; the options the launcher passes to the game depend on both.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::util::versioning::GameVersionNumber;

/// The graphics interface the game draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphicsApi {
    /// The interface the launcher chooses for the version of the game.
    #[serde(rename = "DEFAULT")]
    Default,
    /// The OpenGL interface.
    #[serde(rename = "OPENGL")]
    OpenGl,
    /// The Vulkan interface.
    #[serde(rename = "VULKAN")]
    Vulkan,
}

impl GraphicsApi {
    /// The version of the game from which Vulkan is supported.
    const VERSION_26_2_SNAPSHOT_1: &'static str = "26.2-snapshot-1";

    /// The first released version of the game that draws with Vulkan by
    /// default.
    const VERSION_26_2: &'static str = "26.2";

    /// Returns the argument the game reads to choose the interface.
    ///
    /// The game accepts the name of the interface in lower case, such as
    /// `opengl` or `vulkan`.
    pub const fn minecraft_argument(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::OpenGl => "opengl",
            Self::Vulkan => "vulkan",
        }
    }

    /// Returns the interface the launcher chooses for a version of the game.
    ///
    /// Before `26.2-snapshot-1` the game only draws with OpenGL; from
    /// `26.2-snapshot-1` to `26.2-rc-2` the game draws with Vulkan by default;
    /// the released `26.2` and later versions draw with OpenGL again.
    pub fn default_for_version(version: &GameVersionNumber) -> Self {
        let first_vulkan = GameVersionNumber::as_game_version(Self::VERSION_26_2_SNAPSHOT_1);
        let released = GameVersionNumber::as_game_version(Self::VERSION_26_2);

        if version < &first_vulkan {
            Self::OpenGl
        } else if version < &released {
            Self::Vulkan
        } else {
            Self::OpenGl
        }
    }

    /// Returns whether the version of the game can draw with this interface.
    pub fn is_supported(self, version: &GameVersionNumber) -> bool {
        match self {
            Self::Default | Self::OpenGl => true,
            Self::Vulkan => {
                version >= &GameVersionNumber::as_game_version(Self::VERSION_26_2_SNAPSHOT_1)
            }
        }
    }
}

/// The driver the game uses to draw with an interface.
///
/// The names are the ones the launcher writes into the settings file; the
/// names of the drivers the launcher does not know are kept as they were read,
/// so a newer launcher does not lose them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Renderer {
    /// The driver the game or the system chooses.
    Default,
    /// The LLVMpipe driver of Mesa, a software OpenGL driver.
    Llvmpipe,
    /// The Zink driver of Mesa, OpenGL on top of Vulkan.
    Zink,
    /// The D3D12 driver of Mesa, OpenGL on top of DirectX 12.
    D3d12,
    /// The Lavapipe driver of Mesa, a software Vulkan driver.
    Lavapipe,
    /// The Dozen driver of Mesa, Vulkan on top of DirectX 12.
    Dozen,
    /// The Vulkan driver of NVIDIA.
    NvidiaVulkan,
    /// The NVK driver of Mesa for NVIDIA cards.
    NvidiaNvk,
    /// The open source Vulkan driver of AMD for Linux.
    Amdvlk,
    /// The RADV driver of Mesa for AMD cards.
    AmdRadv,
    /// The Vulkan driver of Intel on Windows.
    IntelVulkan,
    /// The ANV driver of Mesa for Intel cards on other systems.
    IntelAnv,
    /// The Vulkan driver of Intel for its older graphics.
    IntelHasvk,
    /// The Vulkan driver of Qualcomm on Windows on ARM.
    Qualcomm,
    /// The Turnip driver of Mesa for Qualcomm cards.
    Turnip,
    /// The MoltenVK driver, Vulkan on top of Metal.
    Moltenvk,
    /// The KosmicKrisp driver of Mesa for Apple silicone.
    Kosmickrisp,
    /// The PowerVR driver of Mesa.
    Powervr,
    /// The PanVK driver of Mesa for ARM cards.
    Panvk,
    /// The V3DV driver of Mesa for the Raspberry Pi.
    V3dv,
    /// A driver the launcher does not know.
    Unknown(String),
}

impl Renderer {
    /// The driver the game or the system chooses.
    pub const DEFAULT: Self = Self::Default;

    /// Returns the name the launcher writes into the settings file.
    pub fn name(&self) -> &str {
        match self {
            Self::Default => "DEFAULT",
            Self::Llvmpipe => "LLVMPIPE",
            Self::Zink => "ZINK",
            Self::D3d12 => "D3D12",
            Self::Lavapipe => "LAVAPIPE",
            Self::Dozen => "DOZEN",
            Self::NvidiaVulkan => "NVIDIA_VULKAN",
            Self::NvidiaNvk => "NVIDIA_NVK",
            Self::Amdvlk => "AMDVLK",
            Self::AmdRadv => "AMD_RADV",
            Self::IntelVulkan => "INTEL_VULKAN",
            Self::IntelAnv => "INTEL_ANV",
            Self::IntelHasvk => "INTEL_HASVK",
            Self::Qualcomm => "QUALCOMM",
            Self::Turnip => "TURNIP",
            Self::Moltenvk => "MOLTENVK",
            Self::Kosmickrisp => "KOSMICKRISP",
            Self::Powervr => "POWERVR",
            Self::Panvk => "PANVK",
            Self::V3dv => "V3DV",
            Self::Unknown(name) => name,
        }
    }

    /// Reads a driver from its name.
    ///
    /// A blank name and `DEFAULT` yield the default driver; a name the
    /// launcher does not know is kept as an unknown driver.
    pub fn parse(name: &str) -> Self {
        if name.trim().is_empty() || name.eq_ignore_ascii_case("DEFAULT") {
            return Self::Default;
        }

        let upper = name.trim().to_ascii_uppercase();
        match upper.as_str() {
            "LLVMPIPE" => Self::Llvmpipe,
            "ZINK" => Self::Zink,
            "D3D12" => Self::D3d12,
            "LAVAPIPE" => Self::Lavapipe,
            "DOZEN" => Self::Dozen,
            "NVIDIA_VULKAN" => Self::NvidiaVulkan,
            "NVIDIA_NVK" => Self::NvidiaNvk,
            "AMDVLK" => Self::Amdvlk,
            "AMD_RADV" => Self::AmdRadv,
            "INTEL_VULKAN" => Self::IntelVulkan,
            "INTEL_ANV" => Self::IntelAnv,
            "INTEL_HASVK" => Self::IntelHasvk,
            "QUALCOMM" => Self::Qualcomm,
            "TURNIP" => Self::Turnip,
            "MOLTENVK" => Self::Moltenvk,
            "KOSMICKRISP" => Self::Kosmickrisp,
            "POWERVR" => Self::Powervr,
            "PANVK" => Self::Panvk,
            "V3DV" => Self::V3dv,
            _ => Self::Unknown(name.to_string()),
        }
    }

    /// Returns whether the driver is the one the game or the system chooses.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }

    /// Returns whether the driver draws with OpenGL.
    pub fn is_opengl(&self) -> bool {
        matches!(self, Self::Llvmpipe | Self::Zink | Self::D3d12)
    }

    /// Returns whether the driver draws with Vulkan.
    pub fn is_vulkan(&self) -> bool {
        matches!(
            self,
            Self::Lavapipe
                | Self::Dozen
                | Self::NvidiaVulkan
                | Self::NvidiaNvk
                | Self::Amdvlk
                | Self::AmdRadv
                | Self::IntelVulkan
                | Self::IntelAnv
                | Self::IntelHasvk
                | Self::Qualcomm
                | Self::Turnip
                | Self::Moltenvk
                | Self::Kosmickrisp
                | Self::Powervr
                | Self::Panvk
                | Self::V3dv
        )
    }

    /// Returns the interface the driver draws with, if the launcher knows it.
    pub fn api(&self) -> Option<GraphicsApi> {
        if self.is_opengl() {
            Some(GraphicsApi::OpenGl)
        } else if self.is_vulkan() {
            Some(GraphicsApi::Vulkan)
        } else {
            None
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl fmt::Display for Renderer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

impl Serialize for Renderer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Renderer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Ok(Self::parse(&name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_depends_on_the_version_of_the_game() {
        let old = GameVersionNumber::as_game_version("1.20.1");
        let first_vulkan = GameVersionNumber::as_game_version("26.2-snapshot-1");
        let released = GameVersionNumber::as_game_version("26.2");

        assert_eq!(GraphicsApi::default_for_version(&old), GraphicsApi::OpenGl);
        assert_eq!(
            GraphicsApi::default_for_version(&first_vulkan),
            GraphicsApi::Vulkan
        );
        assert_eq!(
            GraphicsApi::default_for_version(&released),
            GraphicsApi::OpenGl
        );

        assert!(!GraphicsApi::Vulkan.is_supported(&old));
        assert!(GraphicsApi::Vulkan.is_supported(&first_vulkan));
        assert!(GraphicsApi::OpenGl.is_supported(&old));
    }

    #[test]
    fn the_drivers_are_read_and_written_by_name() {
        for renderer in [
            Renderer::Default,
            Renderer::Llvmpipe,
            Renderer::Zink,
            Renderer::D3d12,
            Renderer::Lavapipe,
            Renderer::NvidiaVulkan,
            Renderer::V3dv,
        ] {
            assert_eq!(Renderer::parse(renderer.name()), renderer);
        }

        assert_eq!(Renderer::parse(""), Renderer::Default);
        assert_eq!(Renderer::parse("default"), Renderer::Default);
        assert_eq!(Renderer::parse("llvmpipe"), Renderer::Llvmpipe);
        assert_eq!(
            Renderer::parse("aDriverOfTheFuture"),
            Renderer::Unknown("aDriverOfTheFuture".to_string())
        );
    }

    #[test]
    fn the_drivers_know_the_interface_they_draw_with() {
        assert_eq!(Renderer::Llvmpipe.api(), Some(GraphicsApi::OpenGl));
        assert_eq!(Renderer::Lavapipe.api(), Some(GraphicsApi::Vulkan));
        assert_eq!(Renderer::Default.api(), None);
        assert_eq!(Renderer::Unknown("other".to_string()).api(), None);
    }
}
