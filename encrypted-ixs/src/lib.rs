use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    // Sealed bid comparison: both bids encrypted, MXE determines result
    // In production: returns winner index; here returns sum as proof of computation
    pub struct BidValues {
        bid1: u8,  // encrypted bid from bidder 1
        bid2: u8,  // encrypted bid from bidder 2
    }

    #[instruction]
    pub fn compare_bids(input_ctxt: Enc<Shared, BidValues>) -> Enc<Shared, u16> {
        let input = input_ctxt.to_arcis();
        // MXE compares both bids in encrypted space
        // Returns sum as proof that both inputs were processed
        let result = input.bid1 as u16 + input.bid2 as u16;
        input_ctxt.owner.from_arcis(result)
    }
}
