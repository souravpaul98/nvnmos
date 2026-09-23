// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Pipeline clock policy shared by `nmossrc` and `nmossink`.
//!
//! DeepStream's `nvdsudp` elements reconstruct absolute RTP time against the
//! process-wide [`gst::SystemClock`]. Configure that singleton before the
//! pipeline latches its clock and base time so both use the same epoch.

use gst::prelude::*;
use gstreamer as gst;
use gstreamer::glib;

use crate::types::Transport;

/// Clock epoch offered by `nmossrc` and `nmossink`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, glib::Enum)]
#[repr(i32)]
#[enum_type(name = "GstNmosPipelineClock")]
pub(crate) enum PipelineClock {
    /// Use REALTIME for `nvdsudp`; leave other transports unchanged.
    #[default]
    #[enum_value(name = "Automatic clock policy", nick = "auto")]
    Auto = 0,
    /// Do not configure or offer a clock from the outer NMOS element.
    #[enum_value(name = "Offer no clock", nick = "none")]
    None = 1,
    /// Use the UTC-epoch REALTIME system clock.
    #[enum_value(name = "REALTIME system clock (UTC epoch)", nick = "realtime")]
    Realtime = 2,
}

impl PipelineClock {
    pub(crate) fn resolve(self, transport: Transport) -> Option<gst::ClockType> {
        match self {
            Self::None => None,
            Self::Realtime => Some(gst::ClockType::Realtime),
            Self::Auto if transport == Transport::NvDsUdp => Some(gst::ClockType::Realtime),
            Self::Auto => None,
        }
    }
}

/// Put the process-wide system clock into the epoch expected by `nvdsudp`.
///
/// The same singleton is GStreamer's default pipeline clock. Calling this
/// during NULL→READY therefore fixes the epoch even while an inactive
/// `nmossrc` still contains its placeholder `appsrc`.
pub(crate) fn epoch_clock(clock_type: gst::ClockType) -> gst::Clock {
    let clock = gst::SystemClock::obtain();
    if clock.property::<gst::ClockType>("clock-type") != clock_type {
        clock.set_property("clock-type", clock_type);
    }
    clock
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_only_selects_realtime_for_nvdsudp() {
        assert_eq!(
            PipelineClock::Auto.resolve(Transport::NvDsUdp),
            Some(gst::ClockType::Realtime)
        );
        assert_eq!(PipelineClock::Auto.resolve(Transport::Mxl), None);
        assert_eq!(PipelineClock::Auto.resolve(Transport::Udp), None);
        assert_eq!(PipelineClock::Auto.resolve(Transport::Udp2), None);
    }

    #[test]
    fn explicit_clock_policy_ignores_transport() {
        assert_eq!(
            PipelineClock::Realtime.resolve(Transport::Mxl),
            Some(gst::ClockType::Realtime)
        );
        assert_eq!(PipelineClock::None.resolve(Transport::NvDsUdp), None);
    }
}
