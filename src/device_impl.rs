use crate::{
    interface::{I2cInterface, ReadData, SpiInterface, WriteData},
    BitFlags as BF, Config, Error, Lsm303agr, Register, Status, UnscaledMeasurement,
};

impl<I2C> Lsm303agr<I2cInterface<I2C>> {
    /// Create new instance of the LSM303AGR device communicating through I2C.
    pub fn new_with_i2c(i2c: I2C) -> Self {
        Lsm303agr {
            iface: I2cInterface { i2c },
            ctrl_reg1_a: Config { bits: 0x7 },
            ctrl_reg4_a: Config { bits: 0 },
            accel_odr: None,
        }
    }

    /// Destroy driver instance, return I2C bus.
    pub fn destroy(self) -> I2C {
        self.iface.i2c
    }
}

impl<SPI, CSXL, CSMAG> Lsm303agr<SpiInterface<SPI, CSXL, CSMAG>> {
    /// Create new instance of the LSM303AGR device communicating through SPI.
    pub fn new_with_spi(spi: SPI, chip_select_accel: CSXL, chip_select_mag: CSMAG) -> Self {
        Lsm303agr {
            iface: SpiInterface {
                spi,
                cs_xl: chip_select_accel,
                cs_mag: chip_select_mag,
            },
            ctrl_reg1_a: Config { bits: 0x7 },
            ctrl_reg4_a: Config { bits: 0 },
            accel_odr: None,
        }
    }

    /// Destroy driver instance, return SPI bus instance and chip select pin.
    pub fn destroy(self) -> (SPI, CSXL, CSMAG) {
        (self.iface.spi, self.iface.cs_xl, self.iface.cs_mag)
    }
}

impl<DI, CommE, PinE> Lsm303agr<DI>
where
    DI: ReadData<Error = Error<CommE, PinE>> + WriteData<Error = Error<CommE, PinE>>,
{
    /// Initialize registers
    pub fn init(&mut self) -> Result<(), Error<CommE, PinE>> {
        let reg4 = self.ctrl_reg4_a.with_high(BF::BDU);
        self.iface
            .write_accel_register(Register::CTRL_REG4_A, reg4.bits)?;
        self.ctrl_reg4_a = reg4;
        Ok(())
    }

    /// Accelerometer status
    pub fn accel_status(&mut self) -> Result<Status, Error<CommE, PinE>> {
        self.iface
            .read_accel_register(Register::STATUS_REG_A)
            .map(convert_status)
    }

    /// Accelerometer data
    pub fn accel_data(&mut self) -> Result<UnscaledMeasurement, Error<CommE, PinE>> {
        let data = self
            .iface
            .read_accel_3_double_registers(Register::OUT_X_L_A)?;
        let lp_enabled = self.ctrl_reg1_a.is_high(BF::LP_EN);
        let hr_enabled = self.ctrl_reg4_a.is_high(BF::HR);
        let resolution_factor = if hr_enabled {
            1 << 4
        } else if lp_enabled {
            1 << 8
        } else {
            1 << 6
        };
        let x = (data.0 as i16) / resolution_factor;
        let y = (data.1 as i16) / resolution_factor;
        let z = (data.2 as i16) / resolution_factor;
        Ok(UnscaledMeasurement { x, y, z })
    }

    /// Magnetometer status
    pub fn mag_status(&mut self) -> Result<Status, Error<CommE, PinE>> {
        self.iface
            .read_mag_register(Register::STATUS_REG_M)
            .map(convert_status)
    }

    /// Get accelerometer device ID
    pub fn accelerometer_id(&mut self) -> Result<u8, Error<CommE, PinE>> {
        self.iface.read_accel_register(Register::WHO_AM_I_A)
    }

    /// Get magnetometer device ID
    pub fn magnetometer_id(&mut self) -> Result<u8, Error<CommE, PinE>> {
        self.iface.read_mag_register(Register::WHO_AM_I_M)
    }
}

fn convert_status(st: u8) -> Status {
    Status {
        xyz_overrun: (st & BF::XYZOR) != 0,
        z_overrun: (st & BF::ZOR) != 0,
        y_overrun: (st & BF::YOR) != 0,
        x_overrun: (st & BF::XOR) != 0,
        xyz_new_data: (st & BF::XYZDR) != 0,
        z_new_data: (st & BF::ZDR) != 0,
        y_new_data: (st & BF::YDR) != 0,
        x_new_data: (st & BF::XDR) != 0,
    }
}