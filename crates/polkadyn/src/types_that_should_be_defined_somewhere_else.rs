use parity_scale_codec::Decode;

/// A phase of a block's execution.
#[derive(Clone, Decode, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Phase {
    /// Applying an extrinsic.
    ApplyExtrinsic(u32),
    /// Finalizing the block.
    Finalization,
    /// Initializing the block.
    Initialization,
}
