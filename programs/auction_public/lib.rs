// SealedBid Public Auction Layer

#[derive(Clone)]
pub struct AuctionAccount {
    pub creator: [u8; 32],
    pub is_open: bool,
}

#[derive(Clone)]
pub struct EncryptedBidAccount {
    pub bidder: [u8; 32],
    pub encrypted_bid: Vec<u8>,
}

pub fn create_auction(creator: [u8; 32]) -> AuctionAccount {
    AuctionAccount {
        creator,
        is_open: true,
    }
}

pub fn submit_encrypted_bid(
    bidder: [u8; 32],
    encrypted_bid: Vec<u8>,
) -> EncryptedBidAccount {
    EncryptedBidAccount {
        bidder,
        encrypted_bid,
    }
}
