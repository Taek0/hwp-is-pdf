//! HWP distribution document decoder, Hancom distribution specification rev 1.2 §2.
use anyhow::{Result,ensure};
use aes::{Aes128,cipher::{BlockDecrypt,KeyInit}};
pub fn decode(data:&[u8])->Result<Vec<u8>>{
    ensure!(data.len()>=260,"Truncated distribution header");
    let record=u32::from_le_bytes(data[..4].try_into()?);ensure!(record&1023==28&&record>>20==256,"Expected HWPTAG_DISTRIBUTE_DOC_DATA");
    let mut header=data[4..260].to_vec();let seed=u32::from_le_bytes(header[..4].try_into()?);let mut state=seed;
    let mut rand=||{state=state.wrapping_mul(214013).wrapping_add(2531011);(state>>16)&0x7fff};
    let mut p=0;while p<256{let mask=(rand()&255) as u8;let count=(rand()&15)+1;for _ in 0..count{if p==256{break}header[p]^=mask;p+=1;}}
    let start=(seed&15)as usize+4;let cipher=Aes128::new_from_slice(&header[start..start+16])?;
    let mut result=data[260..].to_vec();ensure!(result.len().is_multiple_of(16),"Distribution AES body is not block aligned");
    for block in result.chunks_exact_mut(16){cipher.decrypt_block(aes::cipher::generic_array::GenericArray::from_mut_slice(block));}
    Ok(result)
}
