/******************************************************************************
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.  If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/.
 * 
 * Software distributed under the License is distributed on an "AS IS" basis, 
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for 
 * the specific language governing rights and limitations under the License.
 *
 * The Original Code is: bitstream.rs
 * The Initial Developer of the Original Code is: William Wong (williamw520@gmail.com)
 * Portions created by William Wong are Copyright (C) 2013 William Wong, All Rights Reserved.
 *
 ******************************************************************************/

use std::rt::io::Reader;
use std::rt::io::Writer;
use std::rt::io::Decorator;

#[test]
use std::rt::io::mem::{MemReader,MemWriter};



/// Bit reader to read bits off a passed-in Reader.
/// Supports reading 1 bit or N bits, upto 32 bits at a call.
/// Bytes are consumed one at a time, one after another.
/// Support bit reading direction from LSB or from MSB.
///
/// From LSB (direction_lsb = true), bits are read from the LSB (0th bit) to MSB (7th bit) within a byte.
///                  <----
///  ... FEDCBA98 76543210
///        byte-2   byte-1
///
/// From MSB (direction_lsb = false)
///  ---->
///  01234567 89ABCDEF ...
///  byte-1   byte-2
///
pub struct BitReader<R> {
    bit_buf:    BitBuf,
    reader:     R,
}

impl<R: Reader> BitReader<R> {

    pub fn new(reader: R, direction_lsb: bool) -> BitReader<R> {
        BitReader {
            bit_buf:    BitBuf::new(direction_lsb),
            reader:     reader,
        }
    }

    /// Read a number of bits from the stream, as a u32 value.  bits_to_read is the number of bits to read.
    /// Max bits to read is 32, to fit in the u32 return value.
    pub fn read_bits(&mut self, bits_to_read : uint) -> Option<u32> {
        return self.bit_buf.read_bits(bits_to_read, &mut self.reader);
    }

    /// Consume the remaining bits in the bit buffer of byte-size.  Clear the buffer.  Return the remaining bits.
    /// It has the effect of clearing any bits upto the next byte boundary in the stream.
    /// This will not advance the bit stream beyond the next byte boundary.  Repeated calls will only clear the empty buffer.
    pub fn consume_buf_bits(&mut self) -> Option<u8> {
        if self.bit_buf.get_bit_count() == 0 {
            None
        } else {
            let retval = self.bit_buf.get_remaining_bits();
            self.bit_buf.set_free_bits(8);
            Some(retval)
        }
    }

    /// Return the number of bits buffered.
    pub fn get_bit_count(&self) -> uint {
        return self.bit_buf.get_bit_count();
    }

}

/// Allows reading bytes directly from the inner reader.  Be careful with it.
/// There might be bits remaining in the bit buffer.  Make sure the bit boundary is lined up to byte.
/// Use consume_buf_bits() to get the partial bits first.
impl<R: Reader> Reader for BitReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Option<uint> {
        return self.reader.read(buf);
    }

    fn eof(&mut self) -> bool {
        return self.reader.eof();
    }
}

/// Decorator to access the inner reader
impl<R: Reader> Decorator<R> for BitReader<R> {
    fn inner(self) -> R {
        self.reader
    }

    fn inner_ref<'a>(&'a self) -> &'a R {
        &self.reader
    }

    fn inner_mut_ref<'a>(&'a mut self) -> &'a mut R {
        &mut self.reader
    }
}


/// Bit writer writing bits to the passed-in writer
/// Supports writing 1 bit or N bits, upto 32 bits at a call.
/// Bytes are written one at a time, one after another.
/// Support bit writing direction from LSB or from MSB.
///
/// From LSB (direction_lsb = true), bits are read from the LSB (0th bit) to MSB (7th bit) within a byte.
///                  <----
///  ... FEDCBA98 76543210
///        byte-2   byte-1
///
/// From MSB (direction_lsb = false)
///  ---->
///  01234567 89ABCDEF ...
///  byte-1   byte-2
///
pub struct BitWriter<W> {
    priv bit_buf:   BitBuf,
    priv writer:    W,
}

impl<W: Writer> BitWriter<W> {

    pub fn new(writer: W, direction_lsb: bool) -> BitWriter<W> {
        BitWriter {
            bit_buf:    BitBuf::new(direction_lsb),
            writer:     writer,
        }
    }

    /// Write a number of bits to the writer.  bits_to_write is the number of bits in bit_value to write.
    /// bit_value is the bits to write.  Max bits to write is 32, to fit in the u32 value.
    pub fn write_bits(&mut self, bits_to_write : uint, bit_value: u32) {
        return self.bit_buf.write_bits(bits_to_write, bit_value, &mut self.writer);
    }

    /// Flush the remaining bits in bit_buf.  This writes out a whole byte, including the unused bits with padding 0.
    /// Must be called before closing the writer.  Should be called at 8-bit aligned boundary, or as a last call before close.
    pub fn flush_bits(&mut self) {
        self.bit_buf.flush_bits(&mut self.writer);
    }

}

/// Allows writing bytes directly from the inner writer.  Be careful with it.
/// There might be bits remaining in the bit buffer.  Make sure all partial bits have been flushed before calling this.
impl<W: Writer> Writer for BitWriter<W> {
    fn write(&mut self, buf: &[u8]) {
        return self.writer.write(buf);
    }

    fn flush(&mut self) {
        return self.writer.flush();
    }
}

/// Decorator to access the inner writer
impl<R: Writer> Decorator<R> for BitWriter<R> {
    fn inner(self) -> R {
        self.writer
    }

    fn inner_ref<'a>(&'a self) -> &'a R {
        &self.writer
    }

    fn inner_mut_ref<'a>(&'a mut self) -> &'a mut R {
        &mut self.writer
    }
}


/// Bit buffer for reading and writing bits to streams.  Supports reading or writing 1 bit or N bits, upto 32 bits at a call.
/// A BitBuf struct can only be used either for reading or writing.  Use a separate instance for reading or for writing.
/// The usage is to couple a reader (or writer) with a BitBuf, using the BitBuf to buffer pending bits and doing the read/write logic.
/// Support reading/writing direction from LSB or from MSB.
///
/// From LSB (direction_lsb = true), bits are read from the LSB (0th bit) to MSB (7th bit) within a byte.
///                  <----
///  ... FEDCBA98 76543210
///        byte-2   byte-1
///
/// From MSB (direction_lsb = false)
///  ---->
///  01234567 89ABCDEF ...
///  byte-1   byte-2
///
pub struct BitBuf {
    bit_buf:        u8,         // buffer up 1 byte (8 bits) of data since needs to read at least 1 byte from writer;
    bits_in_buf:    uint,       // number of bits buffered up in bit_buf;
    direction_lsb:  bool,       // direction to read bits, in LSB or MSB order.
}

impl BitBuf {

    pub fn new(direction_lsb: bool) -> BitBuf {
        BitBuf {
            bit_buf:        0u8,
            bits_in_buf:    0,
            direction_lsb:  direction_lsb,
        }
    }

    /// Return the number of bits buffered.
    pub fn get_bit_count(&self) -> uint {
        return self.bits_in_buf;
    }

    /// Return the raw bits currentedly buffered in the buffer byte.  These might be invalid bits that have been read off.
    /// Need to combine with bits_in_buf and direction_lsb to determine the remaining bits.
    pub fn get_bit_buf(&self) -> u8 {
        return self.bit_buf;
    }

    /// Return the actual remaining bits in the buffer.
    pub fn get_remaining_bits(&self) -> u8 {
        if (self.direction_lsb) {
            // Shift down the remainin bits at the high positions by the free bit count.
            return self.bit_buf >> self.get_free_bits();
        } else {
            // Mask off the invalid high bits to keep the valid bits in the low positions.
            return self.bit_buf & ((1 << self.bits_in_buf) - 1);
        }
    }

    /// Return the number of free bit slots in the buffer.
    pub fn get_free_bits(&self) -> uint {
        return 8 - self.bits_in_buf;
    }

    /// Set the number of free bit slots in the buffer.
    fn set_free_bits(&mut self, free_bits: uint) {
        self.bits_in_buf = 8 - free_bits;
    }

    fn clear_buf(&mut self) {
        self.bit_buf = 0;
        self.bits_in_buf = 0;
    }

    pub fn get_direction_lsb(&self) -> bool {
        return self.direction_lsb;
    }


    // Methods for reading.  A BitBuf struct should not be used for both reading and writing.

    /// Read a number of bits from the stream, as a u32 value.  bits_to_read is the number of bits to read.
    /// Max bits to read is 32, to fit in the u32 return value.
    pub fn read_bits<T: Reader>(&mut self, mut bits_to_read : uint, reader: &mut T) -> Option<u32> {
        let mut retval = 0u32;
        let mut byte = [0];

        // Read the next byte into buffer if it is empty and some bits are needed.
        if self.bits_in_buf == 0 && bits_to_read > 0 {
            match reader.read(byte) {
                Some(1) =>  self.bit_buf = byte[0],
                _       =>  return None     // all other conditions are for end of stream
            }
            self.bits_in_buf = 8;
        }

        let mut bits_in_retval = 0u;
        // Keep reading while bits needed are more than bits in the buffer.
        while bits_to_read > self.bits_in_buf {
            // bit_buf is either loaded up with 8 bits or has some bits left over from the previous call.
            // Move all of them to retval.
            if self.direction_lsb {
                // read from LSB: case 1, bits_to_read=5, read off 'Abcde'
                //      buf [87654321 876543Ab cde<<<<<] => retval[00000cde], buf [87654321 876543Ab <<<<<<<<]
                //      break out of loop
                //      buf [87654321 876543Ab <<<<<<<<] => retval[000Abcde], buf [87654321 876543<< <<<<<<<<]
                // read from LSB: case 2, bits_to_read=13, read off 'Abcdefghijklm'
                //      buf [876543Ab cdefghij klm<<<<<] => retval[00000000 00000klm], buf [876543Ab cdefghij <<<<<<<<]
                //      buf [876543Ab cdefghij <<<<<<<<] => retval[00000cde fghijklm], buf [876543Ab <<<<<<<< <<<<<<<<]
                //      break out of loop
                //      buf [876543Ab <<<<<<<< <<<<<<<<] => retval[000Abcde fghijklm], buf [876543<< <<<<<<<< <<<<<<<<]
                let bits_to_move = (self.bit_buf as u32) >> self.get_free_bits();   // shift down to remove invalid bits
                retval = retval | bits_to_move << bits_in_retval;                   // shift up to the high posistion and move into retval
                bits_in_retval += self.bits_in_buf;
                bits_to_read -= self.bits_in_buf;
                self.bits_in_buf = 0;
            } else {
                // read from MSB: case 1, bits_to_read=5, read off 'Abcde'
                //      buf [>>>>>Abc de654321 87654321] => retval[000Abc00], buf [>>>>>>>> de654321 87654321]
                //      break out of loop
                //      buf [>>>>>>>> de654321 87654321] => retval[000Abcde], buf [>>>>>>>> >>654321 87654321]
                // read from MSB: case 2, bits_to_read=13, read off 'Abcdefghijklm'
                //      buf [>>>>>Abc defghijk lm654321] => retval[000Abc00 00000000], buf [>>>>>>>> defghijk lm654321]
                //      buf [>>>>>>>> defghijk lm654321] => retval[000Abcde fghijk00], buf [>>>>>>>> >>>>>>>> lm654321]
                //      break out of loop
                //      buf [>>>>>>>> >>>>>>>> lm654321] => retval[000Abcde fghijklm], buf [>>>>>>>> >>>>>>>> >>654321]
                retval = retval | (self.bit_buf as u32) << (bits_to_read - self.bits_in_buf);
                bits_to_read -= self.bits_in_buf;
                self.bits_in_buf = 0;
            }

            // If more bits are needed, read the next byte into buffer.
            if bits_to_read > 0 {
                match reader.read(byte) {
                    Some(1) =>  self.bit_buf = byte[0],
                    _       =>  return None     // all other conditions are for end of stream
                }
                self.bits_in_buf = 8;
            }
        }

        // bits_to_read is less than or equal 8
        if bits_to_read > 0 {
            if (self.direction_lsb) {
                // from LSB: case 1, bits_to_read=5, read off 'Abcde'
                //      buf [87654321 876543Ab <<<<<<<<] => retval[000Abcde], buf [87654321 876543<< <<<<<<<<]
                // from LSB: case 2, bits_to_read=13, to read off 'Abcdefghijklm'
                //      buf [876543Ab <<<<<<<< <<<<<<<<] => retval[000Abcde fghijklm], buf [876543<< <<<<<<<< <<<<<<<<]
                let lower_mask = (1 << bits_to_read) - 1;
                let bits_to_move = ((self.bit_buf as u32) >> self.get_free_bits()) & lower_mask;    // shift down to remove invalid buf and mask off the high unrelated bits
                retval = retval | bits_to_move << bits_in_retval;
                self.bits_in_buf -= bits_to_read;   // update number of bits left.  bits in buffer don't need to be shifted as they will be read off at their current position in next call.
            } else {
                // read from MSB: case 1, bits_to_read=5, read off 'Abcde'
                //      buf [>>>>>>>> de654321 87654321] => retval[000Abcde], buf [>>>>>>>> >>654321 87654321]
                // read from MSB: case 2, bits_to_read=13, read off 'Abcdefghijklm'
                //      buf [>>>>>>>> >>>>>>>> lm654321] => retval[000Abcde fghijklm], buf [>>>>>>>> >>>>>>>> >>654321]
                let bits_left_over = self.bits_in_buf - bits_to_read;
                retval = retval | (self.bit_buf as u32 >> bits_left_over);  // shift off the left-over bits, OR the high bits over to retval;
                self.bit_buf = self.bit_buf & ((1 << bits_left_over) - 1);  // create bitmask on left-over and remove the copied high bits;
                self.bits_in_buf -= bits_to_read;                           // update number of bits left
            }
        }

        return Some(retval);
    }


    // Methods for writing.  A BitBuf struct instance should not be used for both reading and writing.

    /// Write a number of bits to the writer.  bits_to_write is the number of bits in bit_value to write.
    /// bit_value is the bits to write.  Max bits to write is 32, to fit in the u32 value.
    /// Partial bits might be buffered up in the bit buffer.  Call flush_bits() at the end to flush the byte containing the remaining bits.
    pub fn write_bits<T: Writer>(&mut self, mut bits_to_write : uint, mut bit_value: u32, writer: &mut T) {
        let mut byte = [0];
        bit_value = bit_value & ((1 << bits_to_write) - 1);  // create bitmask with bits_to_write to take only the right bits;

        // Write the buffer out if it's full and more bits are coming.
        if self.bits_in_buf == 8 && bits_to_write > 0 {
            byte[0] = self.bit_buf;
            writer.write(byte);
            self.clear_buf();
        }

        // While the bits_to_write can't fit in the free bit slots in the buffer.
        while bits_to_write >= self.get_free_bits() {
            if (self.direction_lsb) {
                // write from LSB: case 1, bits_to_write=5, write 'Abcde'
                //      write[00000000 000Abcde] buf [00654321] stream [<<<<<<<< <<<<<<<<] => write[00000000 00000Abc] buf [de654321] stream [<<<<<<<< <<<<<<<<]
                //      write[00000000 00000Abc] buf [de654321] stream [<<<<<<<< <<<<<<<<] => write[00000000 00000Abc] buf [00000000] stream [<<<<<<<< de654321]
                //      break out of loop
                //      write[00000000 00000Abc] buf [00000000] stream [<<<<<<<< de654321] => write[00000000 00000000] buf [00000Abc] stream [<<<<<<<< de654321]
                // write from LSB: case 2, bits_to_write=13, write 'Abcdefghijklm'
                //      write[000Abcde fghijklm] buf [00654321] stream [<<<<<<<< <<<<<<<<] => write[00000Abc defghijk] buf [lm654321] stream [<<<<<<<< <<<<<<<<]
                //      write[00000Abc defghijk] buf [lm654321] stream [<<<<<<<< <<<<<<<<] => write[00000Abc defghijk] buf [00000000] stream [<<<<<<<< lm654321]
                //      write[00000Abc defghijk] buf [00000000] stream [<<<<<<<< lm654321] => write[00000000 00000Abc] buf [defghijk] stream [<<<<<<<< lm654321]
                //      write[00000000 00000Abc] buf [defghijk] stream [<<<<<<<< lm654321] => write[00000000 00000Abc] buf [00000000] stream [defghijk lm654321]
                //      break out of loop
                //      write[00000000 00000Abc] buf [00000000] stream [defghijk lm654321] => write[00000000 00000000] buf [00000Abc] stream [defghijk lm654321]
                let bits_to_buf = bit_value & ((1u32 << self.get_free_bits()) - 1); // Take off the lower bits to move into buffer;
                self.bit_buf = 
                    self.bit_buf |                                                  // The buffered bits stays at the lower positions in the 8-bit bit_buf
                    (bits_to_buf << self.bits_in_buf) as u8;                        // The new bits are shifted up to the high bit position of the buffer byte;
                bit_value = bit_value >> self.get_free_bits();
            } else {
                // write from MSB: case 1, bits_to_write=5, write off 'Abcde'
                //      write[00000000 000Abcde] buf [87654300] stream [>>>>>>>> >>>>>>>>] => write[00000000 00000cde] buf [876543Ab] stream [>>>>>>>> >>>>>>>>]
                //      write[00000000 00000cde] buf [876543Ab] stream [>>>>>>>> >>>>>>>>] => write[00000000 00000cde] buf [00000000] stream [876543Ab >>>>>>>>]
                //      break out of loop
                //      write[00000000 00000cde] buf [00000000] stream [876543Ab >>>>>>>>] => write[00000000 00000000] buf [cde00000] stream [876543Ab >>>>>>>>]
                // write from MSB: case 2, bits_to_write=13, write off 'Abcdefghijklm'
                //      write[000Abcde fghijklm] buf [87654300] stream [>>>>>>>> >>>>>>>>] => write[00000cde fghijklm] buf [876543Ab] stream [>>>>>>>> >>>>>>>>]
                //      write[00000cde fghijklm] buf [876543Ab] stream [>>>>>>>> >>>>>>>>] => write[00000cde fghijklm] buf [00000000] stream [876543Ab >>>>>>>>]
                //      write[00000cde fghijklm] buf [00000000] stream [876543Ab >>>>>>>>] => write[00000000 00000klm] buf [cdefghij] stream [876543Ab >>>>>>>>]
                //      write[00000000 00000klm] buf [cdefghij] stream [876543Ab >>>>>>>>] => write[00000000 00000klm] buf [00000000] stream [876543Ab cdefghij]
                //      break out of loop
                //      write[00000000 00000klm] buf [00000000] stream [876543Ab cdefghij] => write[00000000 00000000] buf [klm00000] stream [876543Ab cdefghij]
                let bits_to_buf = bit_value >> (bits_to_write - self.get_free_bits());
                self.bit_buf = self.bit_buf | bits_to_buf as u8;
            }

            bits_to_write -= self.get_free_bits();
            byte[0] = self.bit_buf;
            writer.write(byte);
            self.clear_buf();
        }
        
        // Stuff the remaining bits into the bit_buf.  bits_to_write is less than or equal 8
        if bits_to_write > 0 {
            if (self.direction_lsb) {
                // write from LSB: case 1, bits_to_write=5, write 'Abcde'
                //      write[00000000 00000Abc] buf [00000000] stream [<<<<<<<< de654321] => write[00000000 00000000] buf [00000Abc] stream [<<<<<<<< de654321]
                // write from LSB: case 2, bits_to_write=13, write 'Abcdefghijklm'
                //      write[00000000 00000Abc] buf [00000000] stream [defghijk lm654321] => write[00000000 00000000] buf [00000Abc] stream [defghijk lm654321]
                self.bit_buf = self.bit_buf | (bit_value << self.bits_in_buf) as u8;  // The new bits are shifted up to the high bit position of the buffer byte;
                self.bits_in_buf += bits_to_write;
            } else {
                // write from MSB: case 1, bits_to_write=5, write off 'Abcde'
                //      write[00000000 00000cde] buf [00000000] stream [876543Ab >>>>>>>>] => write[00000000 00000000] buf [cde00000] stream [876543Ab >>>>>>>>]
                // write from MSB: case 2, bits_to_write=13, write off 'Abcdefghijklm'
                //      write[00000000 00000klm] buf [00000000] stream [876543Ab cdefghij] => write[00000000 00000000] buf [klm00000] stream [876543Ab cdefghij]
                self.bit_buf = self.bit_buf | (bit_value as u8 << (self.get_free_bits() - bits_to_write));
                self.bits_in_buf += bits_to_write;
            }
        }
    }

    /// Flush the remaining bits in bit_buf.  This writes out a whole byte, including the unused bits with padding 0.
    /// Must be called before closing the writer.  Should be called at 8-bit aligned boundary, or as a last call before close.
    pub fn flush_bits<T: Writer>(&mut self, writer: &mut T) {
        if self.bits_in_buf > 0 {
            let byte = [ self.bit_buf ];
            writer.write(byte);
            self.clear_buf();
        }
        writer.flush();
    }

}


/// Convert bit string (e.g. "11010001") to binary value
pub fn bitstr_to_value(bit_str: &str) -> u32 {
    let mut retval = 0u32;

    for c in bit_str.iter() {
        retval = retval << 1;
        if (c == '1') {
            retval = retval | 1;
        }
    }

    return retval;
}

/// Convert binary bit value to  bit string (e.g. "11010001")
pub fn value_to_bitstr(mut value: u32) -> ~str {
    let mut s = ~"";

    while value > 0 {
        if value % 2 == 0 {
            s.push_char('0');
        } else {
            s.push_char('1');
        }
        value = value / 2;
    }
    if s.len() == 0 {
        return ~"0";
    }
    return s.rev_iter().collect::<~str>();
}


#[test]
fn test_bitstr_to_value() {
    if bitstr_to_value("11010001") != 0xd1 {
        fail!();
    }
    if bitstr_to_value("0") != 0 {
        fail!();
    }
    if bitstr_to_value("1") != 1 {
        fail!();
    }
    if bitstr_to_value("10") != 2 {
        fail!();
    }
    if bitstr_to_value("11") != 3 {
        fail!();
    }
    if bitstr_to_value("1111") != 0x0F {
        fail!();
    }
    if bitstr_to_value("1111111100001111") != 0xFF0F {
        fail!();
    }
    if bitstr_to_value("1111111100001110") != 0xFF0E {
        fail!();
    }
}

#[test]
fn test_value_to_bitstr() {
    if value_to_bitstr(0xd1) != ~"11010001" {
        fail!();
    }
    if value_to_bitstr(0) != ~"0" {
        fail!();
    }
    if value_to_bitstr(1) != ~"1" {
        fail!();
    }
    if value_to_bitstr(2) != ~"10" {
        fail!();
    }
    if value_to_bitstr(3) != ~"11" {
        fail!();
    }
    if value_to_bitstr(0x0F) != ~"1111" {
        fail!();
    }
    if value_to_bitstr(0xFF0F) != ~"1111111100001111" {
        fail!();
    }
    if value_to_bitstr(0xFF0E) != ~"1111111100001110" {
        fail!();
    }

    for n in range(0u32, 16) {
        let s = value_to_bitstr(n);
        let v = bitstr_to_value(s);
        if (n != v) {
            fail!();
        }
    }
}

#[test]
fn test_read_lsb() {

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(8, mr).unwrap() != bitstr_to_value("11010001") { fail!() }  // byte 1, LSB, 1 1 0 1 0 0 0 1
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }        // byte 2, LSB,             0 1
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("00") { fail!() }        // byte 2, LSB,         0 0
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("101") { fail!() }       // byte 2, LSB,   1 0 1
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }         // byte 2, LSB, 1

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }

    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("00") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("11") { fail!() }

    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("00") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("11") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("001") { fail!() }
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("010") { fail!() }
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("111") { fail!() }
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("000") { fail!() }
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("101") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(4, mr).unwrap() != bitstr_to_value("0001") { fail!() }
    if bb.read_bits(4, mr).unwrap() != bitstr_to_value("1101") { fail!() }

    if bb.read_bits(4, mr).unwrap() != bitstr_to_value("0001") { fail!() }
    if bb.read_bits(4, mr).unwrap() != bitstr_to_value("1101") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(5, mr).unwrap() != bitstr_to_value("10001") { fail!() }
    if bb.read_bits(5, mr).unwrap() != bitstr_to_value("01110") { fail!() }
    if bb.read_bits(5, mr).unwrap() != bitstr_to_value("10100") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(9, mr).unwrap() != bitstr_to_value("111010001") { fail!() }
    if bb.read_bits(9, mr).unwrap() != bitstr_to_value("001101000") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(10, mr).unwrap() != bitstr_to_value("0111010001") { fail!() }
    if bb.read_bits(10, mr).unwrap() != bitstr_to_value("0000110100") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(11, mr).unwrap() != bitstr_to_value("00111010001") { fail!() }
    if bb.read_bits(11, mr).unwrap() != bitstr_to_value("00000011010") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(12, mr).unwrap() != bitstr_to_value("000111010001") { fail!() }
    if bb.read_bits(12, mr).unwrap() != bitstr_to_value("000000001101") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(13, mr).unwrap() != bitstr_to_value("1000111010001") { fail!() }
    if bb.read_bits(13, mr).unwrap() != bitstr_to_value("1100000000110") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(14, mr).unwrap() != bitstr_to_value("01000111010001") { fail!() }
    if bb.read_bits(14, mr).unwrap() != bitstr_to_value("11110000000011") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(15, mr).unwrap() != bitstr_to_value("101000111010001") { fail!() }
    if bb.read_bits(15, mr).unwrap() != bitstr_to_value("111111000000001") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(16, mr).unwrap() != bitstr_to_value("1101000111010001") { fail!() }
    if bb.read_bits(16, mr).unwrap() != bitstr_to_value("1111111100000000") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(true);    // LSB

    if bb.read_bits(17, mr).unwrap() != bitstr_to_value("01101000111010001") { fail!() }
    if bb.read_bits(15, mr).unwrap() != bitstr_to_value("111111110000000") { fail!() }
    if bb.read_bits(1, mr) != None { fail!() }
    if bb.read_bits(2, mr) != None { fail!() }
    if bb.read_bits(3, mr) != None { fail!() }

}

#[test]
fn test_read_msb() {

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(8, mr).unwrap() != bitstr_to_value("11010001") { fail!() }  // byte 1, MSB, 1 1 0 1 0 0 0 1
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("11") { fail!() }        // byte 2, MSB, 1 1
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }        // byte 2, MSB,     0 1
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("000") { fail!() }       // byte 2, MSB,         0 0 0
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }         // byte 2, MSB,               1

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }

    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("1") { fail!() }
    if bb.read_bits(1, mr).unwrap() != bitstr_to_value("0") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("11") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("00") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }

    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("11") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("00") { fail!() }
    if bb.read_bits(2, mr).unwrap() != bitstr_to_value("01") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("110") { fail!() }
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("100") { fail!() }
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("011") { fail!() }
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("101") { fail!() }
    if bb.read_bits(3, mr).unwrap() != bitstr_to_value("000") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(4, mr).unwrap() != bitstr_to_value("1101") { fail!() }
    if bb.read_bits(4, mr).unwrap() != bitstr_to_value("0001") { fail!() }

    if bb.read_bits(4, mr).unwrap() != bitstr_to_value("1101") { fail!() }
    if bb.read_bits(4, mr).unwrap() != bitstr_to_value("0001") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(5, mr).unwrap() != bitstr_to_value("11010") { fail!() }
    if bb.read_bits(5, mr).unwrap() != bitstr_to_value("00111") { fail!() }
    if bb.read_bits(5, mr).unwrap() != bitstr_to_value("01000") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(9, mr).unwrap() != bitstr_to_value("110100011") { fail!() }
    if bb.read_bits(9, mr).unwrap() != bitstr_to_value("101000100") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(10, mr).unwrap() != bitstr_to_value("1101000111") { fail!() }
    if bb.read_bits(10, mr).unwrap() != bitstr_to_value("0100010000") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(11, mr).unwrap() != bitstr_to_value("11010001110") { fail!() }
    if bb.read_bits(11, mr).unwrap() != bitstr_to_value("10001000000") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(12, mr).unwrap() != bitstr_to_value("110100011101") { fail!() }
    if bb.read_bits(12, mr).unwrap() != bitstr_to_value("000100000000") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(13, mr).unwrap() != bitstr_to_value("1101000111010") { fail!() }
    if bb.read_bits(13, mr).unwrap() != bitstr_to_value("0010000000011") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(14, mr).unwrap() != bitstr_to_value("11010001110100") { fail!() }
    if bb.read_bits(14, mr).unwrap() != bitstr_to_value("01000000001111") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(15, mr).unwrap() != bitstr_to_value("110100011101000") { fail!() }
    if bb.read_bits(15, mr).unwrap() != bitstr_to_value("100000000111111") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(16, mr).unwrap() != bitstr_to_value("1101000111010001") { fail!() }
    if bb.read_bits(16, mr).unwrap() != bitstr_to_value("0000000011111111") { fail!() }

    let mut mr = ~MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);    // 11010001 11010001 00000000 11111111;
    let mut bb = BitBuf::new(false);    // MSB

    if bb.read_bits(17, mr).unwrap() != bitstr_to_value("11010001110100010") { fail!() }
    if bb.read_bits(15, mr).unwrap() != bitstr_to_value("000000011111111") { fail!() }
    if bb.read_bits(1, mr) != None { fail!() }
    if bb.read_bits(2, mr) != None { fail!() }
    if bb.read_bits(3, mr) != None { fail!() }

}

#[test]
fn test_reader_lsb() {

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(8).unwrap() != bitstr_to_value("11010001") { fail!() }  // byte 1, LSB, 1 1 0 1 0 0 0 1
    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }        // byte 2, LSB,             0 1
    if br.read_bits(2).unwrap() != bitstr_to_value("00") { fail!() }        // byte 2, LSB,         0 0
    if br.read_bits(3).unwrap() != bitstr_to_value("101") { fail!() }       // byte 2, LSB,   1 0 1
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }         // byte 2, LSB, 1

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }

    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("00") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("11") { fail!() }

    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("00") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("11") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(3).unwrap() != bitstr_to_value("001") { fail!() }
    if br.read_bits(3).unwrap() != bitstr_to_value("010") { fail!() }
    if br.read_bits(3).unwrap() != bitstr_to_value("111") { fail!() }
    if br.read_bits(3).unwrap() != bitstr_to_value("000") { fail!() }
    if br.read_bits(3).unwrap() != bitstr_to_value("101") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(4).unwrap() != bitstr_to_value("0001") { fail!() }
    if br.read_bits(4).unwrap() != bitstr_to_value("1101") { fail!() }

    if br.read_bits(4).unwrap() != bitstr_to_value("0001") { fail!() }
    if br.read_bits(4).unwrap() != bitstr_to_value("1101") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(5).unwrap() != bitstr_to_value("10001") { fail!() }
    if br.read_bits(5).unwrap() != bitstr_to_value("01110") { fail!() }
    if br.read_bits(5).unwrap() != bitstr_to_value("10100") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(9).unwrap() != bitstr_to_value("111010001") { fail!() }
    if br.read_bits(9).unwrap() != bitstr_to_value("001101000") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(10).unwrap() != bitstr_to_value("0111010001") { fail!() }
    if br.read_bits(10).unwrap() != bitstr_to_value("0000110100") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(11).unwrap() != bitstr_to_value("00111010001") { fail!() }
    if br.read_bits(11).unwrap() != bitstr_to_value("00000011010") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(12).unwrap() != bitstr_to_value("000111010001") { fail!() }
    if br.read_bits(12).unwrap() != bitstr_to_value("000000001101") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(13).unwrap() != bitstr_to_value("1000111010001") { fail!() }
    if br.read_bits(13).unwrap() != bitstr_to_value("1100000000110") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(14).unwrap() != bitstr_to_value("01000111010001") { fail!() }
    if br.read_bits(14).unwrap() != bitstr_to_value("11110000000011") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(15).unwrap() != bitstr_to_value("101000111010001") { fail!() }
    if br.read_bits(15).unwrap() != bitstr_to_value("111111000000001") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(16).unwrap() != bitstr_to_value("1101000111010001") { fail!() }
    if br.read_bits(16).unwrap() != bitstr_to_value("1111111100000000") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, true);    // LSB

    if br.read_bits(17).unwrap() != bitstr_to_value("01101000111010001") { fail!() }
    if br.read_bits(15).unwrap() != bitstr_to_value("111111110000000") { fail!() }
    if br.read_bits(1) != None { fail!() }
    if br.read_bits(2) != None { fail!() }
    if br.read_bits(3) != None { fail!() }

}

#[test]
fn test_reader_msb() {

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(8).unwrap() != bitstr_to_value("11010001") { fail!() }  // byte 1, MSB, 1 1 0 1 0 0 0 1
    if br.read_bits(2).unwrap() != bitstr_to_value("11") { fail!() }        // byte 2, MSB, 1 1
    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }        // byte 2, MSB,     0 1
    if br.read_bits(3).unwrap() != bitstr_to_value("000") { fail!() }       // byte 2, MSB,         0 0 0
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }         // byte 2, MSB,               1

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }

    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("1") { fail!() }
    if br.read_bits(1).unwrap() != bitstr_to_value("0") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(2).unwrap() != bitstr_to_value("11") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("00") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }

    if br.read_bits(2).unwrap() != bitstr_to_value("11") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("00") { fail!() }
    if br.read_bits(2).unwrap() != bitstr_to_value("01") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(3).unwrap() != bitstr_to_value("110") { fail!() }
    if br.read_bits(3).unwrap() != bitstr_to_value("100") { fail!() }
    if br.read_bits(3).unwrap() != bitstr_to_value("011") { fail!() }
    if br.read_bits(3).unwrap() != bitstr_to_value("101") { fail!() }
    if br.read_bits(3).unwrap() != bitstr_to_value("000") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(4).unwrap() != bitstr_to_value("1101") { fail!() }
    if br.read_bits(4).unwrap() != bitstr_to_value("0001") { fail!() }

    if br.read_bits(4).unwrap() != bitstr_to_value("1101") { fail!() }
    if br.read_bits(4).unwrap() != bitstr_to_value("0001") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(5).unwrap() != bitstr_to_value("11010") { fail!() }
    if br.read_bits(5).unwrap() != bitstr_to_value("00111") { fail!() }
    if br.read_bits(5).unwrap() != bitstr_to_value("01000") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(9).unwrap() != bitstr_to_value("110100011") { fail!() }
    if br.read_bits(9).unwrap() != bitstr_to_value("101000100") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(10).unwrap() != bitstr_to_value("1101000111") { fail!() }
    if br.read_bits(10).unwrap() != bitstr_to_value("0100010000") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(11).unwrap() != bitstr_to_value("11010001110") { fail!() }
    if br.read_bits(11).unwrap() != bitstr_to_value("10001000000") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(12).unwrap() != bitstr_to_value("110100011101") { fail!() }
    if br.read_bits(12).unwrap() != bitstr_to_value("000100000000") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(13).unwrap() != bitstr_to_value("1101000111010") { fail!() }
    if br.read_bits(13).unwrap() != bitstr_to_value("0010000000011") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(14).unwrap() != bitstr_to_value("11010001110100") { fail!() }
    if br.read_bits(14).unwrap() != bitstr_to_value("01000000001111") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(15).unwrap() != bitstr_to_value("110100011101000") { fail!() }
    if br.read_bits(15).unwrap() != bitstr_to_value("100000000111111") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(16).unwrap() != bitstr_to_value("1101000111010001") { fail!() }
    if br.read_bits(16).unwrap() != bitstr_to_value("0000000011111111") { fail!() }

    let mr = MemReader::new(~[0xd1, 0xd1, 0x00, 0xff]);     // 11010001 11010001 00000000 11111111;
    let mut br = BitReader::new(mr, false);    // MSB

    if br.read_bits(17).unwrap() != bitstr_to_value("11010001110100010") { fail!() }
    if br.read_bits(15).unwrap() != bitstr_to_value("000000011111111") { fail!() }
    if br.read_bits(1) != None { fail!() }
    if br.read_bits(2) != None { fail!() }
    if br.read_bits(3) != None { fail!() }

}


#[test]
fn test_write_lsb() {

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), true);    // LSB;

    bw.write_bits(8, bitstr_to_value("11010001"));  // byte 1, LSB, 1 1 0 1 0 0 0 1
    bw.write_bits(2, bitstr_to_value("01"));        // byte 2, LSB,             0 1
    bw.write_bits(2, bitstr_to_value("00"));        // byte 2, LSB,         0 0
    bw.write_bits(3, bitstr_to_value("101"));       // byte 2, LSB,   1 0 1
    bw.write_bits(1, bitstr_to_value("1"));         // byte 2, LSB, 1
    if bw.inner().inner() != ~[0xd1u8, 0xd1u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), true);    // LSB;
    bw.write_bits(1, 1);
    bw.write_bits(1, 0);
    bw.write_bits(1, 0);
    bw.write_bits(1, 0);
    bw.write_bits(1, 1);
    bw.write_bits(1, 0);
    bw.write_bits(1, 1);
    bw.write_bits(1, 1);

    bw.write_bits(1, 1);
    bw.write_bits(1, 0);
    bw.write_bits(1, 0);
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("001") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), true);    // LSB;
    bw.write_bits(2, bitstr_to_value("01"));
    bw.write_bits(2, bitstr_to_value("00"));
    bw.write_bits(2, bitstr_to_value("01"));
    bw.write_bits(2, bitstr_to_value("11"));

    bw.write_bits(2, bitstr_to_value("01"));
    bw.write_bits(2, bitstr_to_value("00"));
    bw.write_bits(2, bitstr_to_value("01"));
    bw.write_bits(2, bitstr_to_value("11"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), true);    // LSB;
    bw.write_bits(3, bitstr_to_value("001"));
    bw.write_bits(3, bitstr_to_value("010"));
    bw.write_bits(3, bitstr_to_value("111"));
    bw.write_bits(3, bitstr_to_value("000"));
    bw.write_bits(3, bitstr_to_value("101"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("1010001") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), true);    // LSB;
    bw.write_bits(4, bitstr_to_value("0001"));
    bw.write_bits(4, bitstr_to_value("1101"));

    bw.write_bits(4, bitstr_to_value("0001"));
    bw.write_bits(4, bitstr_to_value("1101"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), true);    // LSB;
    bw.write_bits(9, bitstr_to_value("111010001"));
    bw.write_bits(9, bitstr_to_value("001101000"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8, bitstr_to_value("00") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), true);    // LSB;
    bw.write_bits(14, bitstr_to_value("01000111010001"));
    bw.write_bits(14, bitstr_to_value("11110000000011"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8, bitstr_to_value("00000000") as u8, bitstr_to_value("1111") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), true);    // LSB;
    bw.write_bits(17, bitstr_to_value("01101000111010001"));
    bw.write_bits(15, bitstr_to_value("111111110000000"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8, bitstr_to_value("00000000") as u8, bitstr_to_value("11111111") as u8] { fail!() }

}

#[test]
fn test_write_msb() {

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), false);   // MSB;

    bw.write_bits(8, bitstr_to_value("11010001"));  // byte 1, MSB, 1 1 0 1 0 0 0 1
    bw.write_bits(2, bitstr_to_value("11"));        // byte 2, MSB, 1 1
    bw.write_bits(2, bitstr_to_value("01"));        // byte 2, MSB,     0 1
    bw.write_bits(3, bitstr_to_value("000"));       // byte 2, MSB,         0 0 0
    bw.write_bits(1, bitstr_to_value("1"));         // byte 2, MSB,               1
    if bw.inner().inner() != ~[0xd1u8, 0xd1u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), false);   // MSB;
    bw.write_bits(1, 1);
    bw.write_bits(1, 1);
    bw.write_bits(1, 0);
    bw.write_bits(1, 1);
    bw.write_bits(1, 0);
    bw.write_bits(1, 0);
    bw.write_bits(1, 0);
    bw.write_bits(1, 1);

    bw.write_bits(1, 1);
    bw.write_bits(1, 1);
    bw.write_bits(1, 0);
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11000000") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), false);   // MSB;
    bw.write_bits(2, bitstr_to_value("11"));
    bw.write_bits(2, bitstr_to_value("01"));
    bw.write_bits(2, bitstr_to_value("00"));
    bw.write_bits(2, bitstr_to_value("01"));

    bw.write_bits(2, bitstr_to_value("11"));
    bw.write_bits(2, bitstr_to_value("01"));
    bw.write_bits(2, bitstr_to_value("00"));
    bw.write_bits(2, bitstr_to_value("01"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), false);   // MSB;
    bw.write_bits(3, bitstr_to_value("110"));
    bw.write_bits(3, bitstr_to_value("100"));
    bw.write_bits(3, bitstr_to_value("011"));
    bw.write_bits(3, bitstr_to_value("101"));
    bw.write_bits(3, bitstr_to_value("000"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010000") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), false);   // MSB;
    bw.write_bits(4, bitstr_to_value("1101"));
    bw.write_bits(4, bitstr_to_value("0001"));

    bw.write_bits(4, bitstr_to_value("1101"));
    bw.write_bits(4, bitstr_to_value("0001"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), false);   // MSB;
    bw.write_bits(9, bitstr_to_value("110100011"));
    bw.write_bits(9, bitstr_to_value("101000100"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8, bitstr_to_value("00000000") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), false);   // MSB;
    bw.write_bits(14, bitstr_to_value("11010001110100"));
    bw.write_bits(14, bitstr_to_value("01000000001111"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8, bitstr_to_value("00000000") as u8, bitstr_to_value("11110000") as u8] { fail!() }

    // [0xd1, 0xd1, 0x00, 0xff]                             // 11010001 11010001 00000000 11111111;
    let mut bw = BitWriter::new(MemWriter::new(), false);   // MSB;
    bw.write_bits(17, bitstr_to_value("11010001110100010"));
    bw.write_bits(15, bitstr_to_value("000000011111111"));
    bw.flush_bits();
    if bw.inner().inner() != ~[bitstr_to_value("11010001") as u8, bitstr_to_value("11010001") as u8, bitstr_to_value("00000000") as u8, bitstr_to_value("11111111") as u8] { fail!() }

}

