use std::{io::Read, time};

use serialport::{self, SerialPort};


pub const FRAME_STANDARD_MASK: u32 = 0x07_FF;
pub const FRAME_EXTENDED_MASK: u32 = 0x1F_FF_FF_FF;
pub const MAX_FRAME_DLC: usize = 8;

pub const FRAME_START: u8 = 0xAA;
pub const FRAME_END: u8 = 0x55;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FrameType {
    DataFrame,
    RemoteFrame
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FrameIdType {
    StandardFrame,
    ExtendedFrame
}
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Baudrate {
    Baud1000k,
    Baud800k,
    Baud500k,
    Baud400k,
    Baud250k,
    Baud200k,
    Baud125k,
    Baud100k,
    Baud50k,
    Baud20k,
    Baud10k,
    Baud5k
}

impl From<Baudrate> for u8 {
    fn from(value: Baudrate) -> Self {
        match value {
            Baudrate::Baud1000k => 0x01u8,
            Baudrate::Baud800k => 0x02u8,
            Baudrate::Baud500k => 0x03u8,
            Baudrate::Baud400k => 0x04u8,
            Baudrate::Baud250k => 0x05u8,
            Baudrate::Baud200k => 0x06u8,
            Baudrate::Baud125k => 0x07u8,
            Baudrate::Baud100k => 0x08u8,
            Baudrate::Baud50k => 0x09u8,
            Baudrate::Baud20k => 0x0Au8,
            Baudrate::Baud10k => 0x0Bu8,
            Baudrate::Baud5k => 0x0Cu8,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FrameConstructionError {
    TooMuchData,
    FrameIdMismacthFrameIdType,
}

pub struct Frame {
    frame_id: u32,
    frame_type: FrameType,
    frame_id_type: FrameIdType,
    data: [u8; 8],
    dlc: u8
}

impl Frame {
    pub fn new(
        frame_id: u32,
        frame_id_type: FrameIdType,
        frame_type: FrameType,
        data: &[u8]
    ) -> Result<Frame, FrameConstructionError>
    {
        if data.len() > MAX_FRAME_DLC {
            return Result::Err(FrameConstructionError::TooMuchData);
        } else {
            match frame_id_type {
                FrameIdType::StandardFrame => {
                    if frame_id > FRAME_STANDARD_MASK {
                        return Result::Err(FrameConstructionError::FrameIdMismacthFrameIdType);
                    }
                },
                FrameIdType::ExtendedFrame => {
                    if frame_id > FRAME_EXTENDED_MASK {
                        return Result::Err(FrameConstructionError::FrameIdMismacthFrameIdType);
                    }
                }
            }
            
            let mut frame_data = [0u8; 8];
            for (i, v) in data.into_iter().enumerate() {
                frame_data[i] = *v;
            }

            return Result::Ok(
                Frame {
                    frame_id: frame_id,
                    frame_id_type: frame_id_type,
                    frame_type: frame_type,
                    data: frame_data,
                    dlc: data.len() as u8,
                }
            )
        }
    }

    fn frame_id(&self) -> u32 {
        match self.frame_id_type {
            FrameIdType::ExtendedFrame => {
                return self.frame_id & FRAME_EXTENDED_MASK;
            },
            FrameIdType::StandardFrame => {
                return self.frame_id & FRAME_STANDARD_MASK
            }
        }
    }

    fn frame_type(&self) -> FrameType {
        return self.frame_type;
    }

    fn frame_id_type(&self) -> FrameIdType {
        return self.frame_id_type;
    }

    fn data(&self) -> &[u8] {
        return &self.data;
    }

    fn mut_data(&mut self) -> &mut [u8] {
        return &mut self.data;
    }
}

pub enum OperationMode {
    Normal,
    Loopback,
    Silent,
    LoopbackAndSilent
}

impl From<OperationMode> for u8 {
    fn from(value: OperationMode) -> Self {
        match value {
            OperationMode::Normal => 0x00u8,
            OperationMode::Loopback => 0x01u8,
            OperationMode::Silent => 0x02u8,
            OperationMode::LoopbackAndSilent => 0x03u8,
        }
    }
}


pub struct SerialCanSocket {
    serial: Box<dyn serialport::SerialPort>,
    rx_error_counter: u8,
    tx_error_counter: u8
}

impl SerialCanSocket {
    pub fn open<'a>(path: impl Into<std::borrow::Cow<'a, str>>) -> Result<SerialCanSocket, ()> {
        let serial = 
            serialport::new(path, 2000000)
            .data_bits(serialport::DataBits::Eight)
            .stop_bits(serialport::StopBits::One)
            .parity(serialport::Parity::None)
            .flow_control(serialport::FlowControl::None)
            // .timeout(time::Duration::from_millis(50))
            .open();

        let mut serial = match serial {
            Err(_) => {
                return Err(());
            }
            Ok(serial) => {
                serial
            }
        };

        let mut buffer = [
            0xAAu8, 0x55u8, 0x12u8,
            Baudrate::Baud125k.into(),
            0x02,
            0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8,
            OperationMode::Normal.into(),
            1u8,
            0u8, 0u8, 0u8, 0u8,
            0u8
        ];

        let checksum: u16 = [0x12u8,
            Baudrate::Baud125k.into(),
            0x02u8,
            0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8,
            OperationMode::Normal.into(),
            1u8,
            0u8, 0u8, 0u8, 0u8].as_slice().iter().map(|x| (*x as u16)).sum();
        let checksum = (checksum & 0x00FF).try_into().unwrap();
        buffer[19] = checksum;
        let err = serial.write(&buffer);
        println!("{:?}", err);

        return Result::Ok(
            SerialCanSocket {
                serial: serial,
                rx_error_counter: 0u8,
                tx_error_counter: 0u8
            }
        );

    }

    pub fn send(&mut self, frame: &Frame) {
        // println!("send");
        let buffer = [
            0xAA,
            0b1110_0000 | frame.dlc,
            0x44u8,
            0x33u8,
            0x22u8,
            0x11u8,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x55
        ];
        
        let err = self.serial.write(&buffer);
        println!("{:?}", err);
    }

    pub fn read(&mut self) {
        let mut buf = [0u8; 100];
        println!("read");
        let s = self.serial.read(&mut buf).unwrap_or(0);
        println!("s={}", s);
        for j in 0..s {
            println!("{:02X}", buf[j]);
        }
        println!();
    }

    pub fn status(&mut self) {
        let mut buffer = [
            0xAAu8, 0x55u8, 0x04u8,
            0x00,
            0x00,
            0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8,
            0u8,
            0u8,
            0u8, 0u8, 0u8, 0u8,
            0u8
        ];
        let checksum: u16 = [0x04u8,
            0x00u8,
            0x00u8,
            0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8,
            0u8,
            0u8,
            0u8, 0u8, 0u8, 0u8].as_slice().iter().map(|x| (*x as u16)).sum();
        let checksum = (checksum & 0x00FF).try_into().unwrap();
        buffer[19] = checksum;
        let err = self.serial.write(&buffer);
        println!("{:?}", err);

        let mut buf = [0u8; 20];
        println!("status");
        let s = self.serial.read(&mut buf).unwrap_or(0);
        for j in 0..s {
            println!("{:02X}", buf[j]);
        }
        println!();
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
