use anchor_lang::prelude::*;

declare_id!("SBid1111111111111111111111111111111111111111");

/// SealedBid — Confidential auction execution via Arcium MXE
///
/// Bidders submit encrypted bids. Arcium MXE determines the winner
/// without revealing any losing bids. On-chain: only the winner
/// and clearing price are published.
#[program]
pub mod sealedbid {
    use super::*;

    /// Create a sealed bid auction
    pub fn create_auction(
        ctx: Context<CreateAuction>,
        auction_id: u64,
        item_description: String,
        reserve_price_lamports: u64,
        bidding_ends_at: i64,
        mxe_cluster_offset: u64,
    ) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        auction.authority = ctx.accounts.authority.key();
        auction.auction_id = auction_id;
        auction.item_description = item_description;
        auction.reserve_price_lamports = reserve_price_lamports;
        auction.bidding_ends_at = bidding_ends_at;
        auction.mxe_cluster_offset = mxe_cluster_offset;
        auction.bid_count = 0;
        auction.status = AuctionStatus::Active;
        auction.winner = None;
        auction.clearing_price = 0;

        emit!(AuctionCreated {
            auction_id,
            bidding_ends_at,
            mxe_cluster_offset,
        });
        Ok(())
    }

    /// Submit an encrypted sealed bid.
    /// Bid amount is encrypted with MXE public key — only revealed to MXE.
    pub fn submit_bid(
        ctx: Context<SubmitBid>,
        auction_id: u64,
        encrypted_bid: Vec<u8>,
        bid_commitment: [u8; 32],
    ) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        require!(auction.status == AuctionStatus::Active, SealedBidError::AuctionNotActive);
        require!(encrypted_bid.len() <= 128, SealedBidError::BidDataTooLarge);

        let clock = Clock::get()?;
        require!(clock.unix_timestamp < auction.bidding_ends_at, SealedBidError::BiddingClosed);

        let bid = &mut ctx.accounts.bid_record;
        bid.bidder = ctx.accounts.bidder.key();
        bid.auction_id = auction_id;
        bid.encrypted_bid = encrypted_bid;
        bid.bid_commitment = bid_commitment;
        bid.submitted_at = clock.unix_timestamp;

        auction.bid_count += 1;

        emit!(BidSubmitted {
            bidder: ctx.accounts.bidder.key(),
            auction_id,
            bid_commitment,
        });
        Ok(())
    }

    /// Finalize auction with MXE-determined winner.
    /// MXE processes all encrypted bids and returns winning bidder + price.
    pub fn finalize_auction(
        ctx: Context<FinalizeAuction>,
        auction_id: u64,
        winner: Pubkey,
        clearing_price: u64,
        mxe_proof_hash: [u8; 32],
    ) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        require!(auction.auction_id == auction_id, SealedBidError::AuctionMismatch);
        require!(clearing_price >= auction.reserve_price_lamports, SealedBidError::BelowReserve);

        auction.winner = Some(winner);
        auction.clearing_price = clearing_price;
        auction.mxe_proof_hash = mxe_proof_hash;
        auction.status = AuctionStatus::Finalized;

        emit!(AuctionFinalized {
            auction_id,
            winner,
            clearing_price,
            mxe_proof_hash,
        });
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(auction_id: u64)]
pub struct CreateAuction<'info> {
    #[account(init, payer = authority, space = Auction::LEN,
        seeds = [b"auction", &auction_id.to_le_bytes()], bump)]
    pub auction: Account<'info, Auction>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(auction_id: u64)]
pub struct SubmitBid<'info> {
    #[account(mut, seeds = [b"auction", &auction_id.to_le_bytes()], bump)]
    pub auction: Account<'info, Auction>,
    #[account(init, payer = bidder, space = BidRecord::LEN,
        seeds = [b"bid", &auction_id.to_le_bytes(), bidder.key().as_ref()], bump)]
    pub bid_record: Account<'info, BidRecord>,
    #[account(mut)]
    pub bidder: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(auction_id: u64)]
pub struct FinalizeAuction<'info> {
    #[account(mut, seeds = [b"auction", &auction_id.to_le_bytes()], bump, has_one = authority)]
    pub auction: Account<'info, Auction>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Auction {
    pub authority: Pubkey,
    pub auction_id: u64,
    pub item_description: String,       // max 128 bytes
    pub reserve_price_lamports: u64,
    pub bidding_ends_at: i64,
    pub mxe_cluster_offset: u64,
    pub bid_count: u64,
    pub status: AuctionStatus,
    pub winner: Option<Pubkey>,
    pub clearing_price: u64,
    pub mxe_proof_hash: [u8; 32],
}
impl Auction {
    pub const LEN: usize = 8 + 32 + 8 + (4+128) + 8 + 8 + 8 + 8 + 1 + (1+32) + 8 + 32;
}

#[account]
pub struct BidRecord {
    pub bidder: Pubkey,
    pub auction_id: u64,
    pub encrypted_bid: Vec<u8>,   // max 128 bytes
    pub bid_commitment: [u8; 32],
    pub submitted_at: i64,
}
impl BidRecord {
    pub const LEN: usize = 8 + 32 + 8 + (4+128) + 32 + 8;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum AuctionStatus { Active, Finalized, Cancelled }

#[event]
pub struct AuctionCreated { pub auction_id: u64, pub bidding_ends_at: i64, pub mxe_cluster_offset: u64 }
#[event]
pub struct BidSubmitted { pub bidder: Pubkey, pub auction_id: u64, pub bid_commitment: [u8; 32] }
#[event]
pub struct AuctionFinalized { pub auction_id: u64, pub winner: Pubkey, pub clearing_price: u64, pub mxe_proof_hash: [u8; 32] }

#[error_code]
pub enum SealedBidError {
    #[msg("Auction is not active")]
    AuctionNotActive,
    #[msg("Bidding period has closed")]
    BiddingClosed,
    #[msg("Bid data exceeds 128 bytes")]
    BidDataTooLarge,
    #[msg("Auction ID mismatch")]
    AuctionMismatch,
    #[msg("Clearing price is below reserve")]
    BelowReserve,
}
