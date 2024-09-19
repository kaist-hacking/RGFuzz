use crc::{Crc, CRC_32_ISO_HDLC};


fn main(){
    let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);

    let mut digest = crc.digest();

    let val1 : u32 = 123;
    let val2 : u32 = 456;
    let val3 : u32 = 789;

    digest.update(&val1.to_le_bytes());
    digest.update(&val2.to_le_bytes());
    digest.update(&val3.to_le_bytes());

    println!("{:x}", digest.finalize());
}
