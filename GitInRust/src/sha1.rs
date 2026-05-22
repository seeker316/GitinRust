use std::convert::TryInto;

// fn main() {
//     let input = b"hellowold";
//
//     println!("Input string: {}", std::str::from_utf8(input).unwrap());
//     let mut output = [0u8; 20];
//
//     sha1(&mut output, input);
//
//     for byte in &output {
//         print!("{:02x}",byte);
//     }
//     println!();
// }

struct Sha1Ctx {
    state: [u32; 5],
    count: [u32; 2],
    buffer: [u8; 64]
}

pub fn sha1(hash_out: &mut [u8; 20], input: &[u8]){
    
    let mut ctx = Sha1Ctx {
        state: [0; 5],
        count: [0; 2],
        buffer: [0; 64],
    };

    sha1_init(&mut ctx);


    sha1_update(&mut ctx, input, input.len() as u32);


    sha1_final(hash_out, &mut ctx);
}

fn sha1_init (context: &mut Sha1Ctx) {
    
    context.state[0] = 0x67452301;
    context.state[1] = 0xEFCDAB89;
    context.state[2] = 0x98BADCFE;
    context.state[3] = 0x10325476;
    context.state[4] = 0xC3D2E1F0;
    context.count = [0,0];
}

fn sha1_update (context : &mut Sha1Ctx, data: &[u8], len :u32){
    let mut i : usize;
    let mut j : usize;

    let old = context.count[0];
    context.count[0] = context.count[0].wrapping_add(len << 3);
    // checking overflow if the number of bits becomes, less after adding then, it means a buffer
    // overflow has happened, hence incrementing context[1], which stores the upper bits. note that
    // this overflow condition checks for an overflow after the addition
    if context.count[0] < old{
        context.count[1]+= 1;
    }
    
    // this statement here means, (len << 3) >> 32, i.e (len * 8) / 32. to check for overflow after getting the total number of bits, the of condition on top is for handling overflow after adding, the below one is to handle overflow after multiplying len by 8.                                  
    context.count[1] = context.count[1].wrapping_add(len >> 29);

    j = ((old >> 3) & 63) as usize; //checks how many bytes of data are sitting in the block
    
    let len = len as usize;
    if (j + len) > 63{   
        i = 64 - j; //how many bytes needed to fill the buffer
        context.buffer[j..j + i].copy_from_slice(&data[..i]); //copying first i bytes from
                                                              //data...to context buffer...staring
                                                              //from j which is the index/counter
                                                              //for the buffer...to j+1
        sha1_transform(&mut context.state, &context.buffer);
 
        while i + 63 < len {
            sha1_transform(&mut context.state, &data[i..i + 64]);
            i += 64;
        }

        j = 0;
        
    }
    else{
        i = 0; //if it is entering the else then this means, that the buffer was empty..so that's
               //indexing i = 0 and filling the full buffer below
    }

    context.buffer[j..j + (len-i)].copy_from_slice(&data[i..len]);
}

fn sha1_transform(state: &mut [u32; 5], buffer: &[u8]){
    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    
    let mut w = [0u32; 80];

    //buffer is a byte array, containing 64 bytes, chunk exact, splits into 4 byte pieces, here we
    //have 54 bytes, and are splitting it into 4 byte chunks, so the toal number of chunks will be
    //16 chunks, .enumerate adds indexes to those chunks, chunk.try_into().unwrap(), converts slice
    //   into a [u8; 4] array, unwrap() is used because try_into() doesn’t give you the value directly—it gives a Result, and you need to extract the [u8; 4] from it.
    //   :: is called the path seperator operator, and is used to access items inside a namespace,
    //   here it is being used to extract u32 from the result

    for (i, chunk) in buffer.chunks_exact(4).enumerate(){
        w[i] = u32::from_be_bytes(chunk.try_into().unwrap());
    }

    //SHA-1 stretches those 16 words into 80 words, 80 rounds = design balance Enough for strong mixing, Still efficient on hardware Matches the 80-word expansion Structured as 4 × 20 phases for varied mixing. 
    // Instead of shifting bits and losing data, it rotates them. while rotating left, the msb will
    // move to the lsb position and the rest bits will move by one step towards left.
    // The SHA-1 Formula, W[i] = ROL(W[i−3]⊕W[i−8]⊕W[i−14]⊕W[i−16], 1)
    for i in 16..80{
        w[i] = (w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16]).rotate_left(1);
    }
// the below match returns a tuple, for each range, in steps for 20, each tuple consists of a non
// linear mixing function, and a constant. the constants prevent symmetry so patterns don't exist,
// and each round group behaves differently. 
// Each phase uses a different logic function.
//  - 0-19 : choose function, (b & c) | ((!b) & d), what it does If b = 1 → pick bit from c, If b = 0 → pick bit from d. so b "chooses" between c and d. Purpose : conditional behaviour (like branching)
//  - 20–39 and 60–79: XOR, Parity function, Bit is 1 if an odd number of inputs are 1 Purpose :
//  Balanced Mixing
//  - 40-59 : Majority function, Result bit = value held by at least 2 of the 3 inputs. it returns, the majority bit. Purpose : Strong correlation blending.    
    for i in 0..80{
        let (f,k) = match i{
            0..=19 => ((b & c) | ((!b) & d), 0x5A827999), //choose function
            20..=39 => (((b ^ c) ^ d), 0x6ED9EBA1), //XOR / parity function
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC), //majority function
            _ =>  (((b ^ c) ^ d), 0xCA62C1D6),
        };

// for each round from i = 0-79, there is a step formula, this is the heart of the sha-1, then
// shift e = d, d = c, c = rotate_left(b,30), b = a, a = temp, the fucntion changes per round for
// different ranges in steps of 20 as described earlier. 
        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(w[i]);

        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;

    }
    // after 80 rounds, add back to the states.
    
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
 

}

// last step of the sha1, in the end,
// - pad the data, append the total length do one final transform and extract the result.
fn sha1_final(digest: &mut [u8; 20], context : &mut Sha1Ctx)
{
    let mut finalcount = [0u8; 8];
    // converting bit count to bytes, the for loop is to ensure big endian order, high bits first
    // order
       
    // the i & 3 cycles like i = 0 then 1,2,3 then back to 0.
    // ((3 - (i & 3)) * 8), gives shift amounts this cycles like for i = 0, 24, then for i = 1,
    // 16, then 8 then 0. , the >> shift gets the desired byte to the rightmost position, then
    // & 255 keeps the lowest byte. basically we are shifting, each byte of data to the
    // rightmost position in each step, and then storing them in finalcount.
    for i in 0..8{
      let idx = if i >= 4 { 0 } else { 1 };
      finalcount[i] = (context.count[idx] >> (((3 - (i & 3)) * 8) & 255)) as u8;
   }
 
    
    //now we are padding the message, before finalizing the message, the message must be padded
    //such that, total length = 448 bits + 64 bits (8 bytes reserved for length) = 512 bit block,
    //64 bytes.
    // appending single 1 bit for padding
    let mut c: u8 = 0x80;
    sha1_update(context, &[c], 1);

    // adding zeros until length mod 512 = 448, & 504 masks out, the lower 3 bits and keeps the next 6
    // bits. so effectively it is count[0] % 512. 
    while (context.count[0] & 504) != 448{
        c = 0;
        sha1_update(context, &[c], 1);

    }

    //appending original message length.
    sha1_update(context, &finalcount, 8);
    
    // Extracting the hash, here we are converting 5 integers into 20 bytes.
    // which bit word?
    // i >> 2 : each 32 bit word gives 4 bytes, for i = 0-3, i >> 2 becomes 1, for i = 4-7, , i >>
    // 2, it becomes 2, for i = 8-11 it becomes 3 and so on.

    // which byte inside that word?
    // i & 3, cycles in 0,1,2,3....and so on, extracts the msb by shifitng and then extracts the
    // lowes bytes by & 255, this is mentioned in more detail above. (((3 - (i & 3)) * 8) & 255)
    for i in 0..20{
        digest[i] = ((context.state[i >> 2] >> (((3 - (i & 3)) * 8) & 255))) as u8;
    }


}
