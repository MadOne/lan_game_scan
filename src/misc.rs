use std::collections::VecDeque;

// Pop $number bytes from vector.
// When number == 0 pop a 0 terminated string
pub fn pop_bytes(byte_vec: &mut VecDeque<u8>, number: i32) -> Vec<u8> {
    let mut myreturn: Vec<u8> = vec![];

    // Pop a 0 terminated string
    if number == 0 {
        loop {
            let mybyte = byte_vec.pop_front().unwrap();
            if mybyte.to_owned() == 0x00 {
                return myreturn;
            } else {
                myreturn.push(mybyte.to_owned());
            }
        }
    } else
    // Pop $number bytes
    {
        for _n in 1..number + 1 {
            let mybyte = byte_vec.pop_front().unwrap();
            myreturn.push(mybyte.to_owned());
        }
        return myreturn;
    }
}
