use crate::seq::{Buf, Sequential};

/// A physical register for holding data.
///
/// # Examples
///
/// ```
/// # use ferisc::core::reg::phys::PhysReg;
/// # use ferisc::seq::Sequential;
/// // We can construct a register
/// let mut reg = PhysReg::<i32>::new();
///
/// // We can read the register
/// assert_eq!(reg.try_read().copied(), Some(i32::default()));
///
/// // We can write to the register
/// reg.write(2);
/// reg.finish_cycle();
/// assert_eq!(reg.try_read().copied(), Some(2));
///
/// // We can mark a register as "busy" to prevent reads until a new value is written
/// reg.mark_busy();
/// reg.finish_cycle();
/// assert_eq!(reg.try_read(), None, "reading a busy register should not be possible");
/// reg.write(3);
/// reg.finish_cycle();
/// assert_eq!(reg.try_read().copied(), Some(3), "writing a new value should mark the register as not busy");
///
/// // We can also create a hardwired register which ignores writes
/// let mut hard_reg = PhysReg::<i32>::hardwired();
/// assert_eq!(hard_reg.try_read().copied(), Some(i32::default()));
/// hard_reg.write(1);
/// hard_reg.finish_cycle();
/// assert_eq!(hard_reg.try_read().copied(), Some(i32::default()));
/// ```
#[derive(Debug, Clone)]
pub struct PhysReg<T> {
    buf: Buf<Option<T>>,
    hardwired: bool,
}

impl<T: Default> PhysReg<T> {
    /// Creates a new physical register.
    pub fn new() -> Self {
        Self {
            buf: Buf::new(Some(T::default())),
            hardwired: false,
        }
    }

    /// Creates a new hardwired register, which will always return the default value when read.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::core::reg::phys::PhysReg;
    /// # use ferisc::seq::Sequential;
    /// let mut hard_reg = PhysReg::<i32>::hardwired();
    /// assert_eq!(hard_reg.try_read_copied(), Some(i32::default()));
    /// hard_reg.write(1);
    /// hard_reg.finish_cycle();
    /// assert_eq!(hard_reg.try_read_copied(), Some(i32::default()));
    /// ```
    pub fn hardwired() -> Self {
        Self {
            hardwired: true,
            ..Self::new()
        }
    }

    /// Tries to read the register, returning a reference to the data currently
    /// stored if the register is not ["busy"](Self::mark_busy), or [`None`] otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::core::reg::phys::PhysReg;
    /// # use ferisc::seq::Sequential;
    /// let mut reg = PhysReg::<i32>::new();
    /// assert_eq!(reg.try_read().copied(), Some(i32::default()));
    ///
    /// // Reading a "busy" register yields no value
    /// reg.mark_busy();
    /// reg.finish_cycle();
    /// assert_eq!(reg.try_read(), None);
    /// ```
    pub fn try_read(&self) -> Option<&T> {
        self.buf.read().as_ref()
    }

    /// Tries to read the register, returning a copy of the data currently
    /// stored if the register is not ["busy"](Self::mark_busy), or [`None`] otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::core::reg::phys::PhysReg;
    /// # use ferisc::seq::Sequential;
    /// let mut reg = PhysReg::<i32>::new();
    /// assert_eq!(reg.try_read_copied(), Some(i32::default()));
    ///
    /// // Reading a "busy" register yields no value
    /// reg.mark_busy();
    /// reg.finish_cycle();
    /// assert_eq!(reg.try_read_copied(), None);
    /// ```
    pub fn try_read_copied(&self) -> Option<T>
    where
        T: Copy,
    {
        self.buf.read_copied()
    }

    /// Schedules a new value to be written into the register at the end
    /// of the current clock cycle, if it is not hardwired.
    ///
    /// Writing a ["busy"](Self::mark_busy) register will mark it as not busy
    /// in the next clock cycle.
    ///
    /// <div class="warning">
    ///
    /// This method should only be called once per clock cycle, but for
    /// performance reasons, this is only enforced when compiling with
    /// `debug_assertions` enabled. Make sure all call sites conform to this
    /// requirement.
    ///
    /// </div>
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::core::reg::phys::PhysReg;
    /// # use ferisc::seq::Sequential;
    /// let mut reg = PhysReg::<i32>::new();
    /// reg.write(2);
    ///
    /// // Reading the register before the end of
    /// // a clock cycle yields the old data
    /// assert_eq!(reg.try_read().copied(), Some(i32::default()));
    ///
    /// reg.finish_cycle();
    /// assert_eq!(reg.try_read().copied(), Some(2));
    ///
    /// // Writing a "busy" register will mark it as not busy
    /// reg.mark_busy();
    /// reg.finish_cycle();
    /// assert_eq!(reg.try_read(), None);
    /// reg.write(3);
    /// reg.finish_cycle();
    /// assert_eq!(
    ///     reg.try_read().copied(),
    ///     Some(3),
    ///     "writing a new value should mark the register as not busy"
    /// );
    ///
    /// // Writing to a hardwired register has no effect
    /// let mut hard_reg = PhysReg::<i32>::hardwired();
    /// assert_eq!(hard_reg.try_read().copied(), Some(i32::default()));
    /// hard_reg.write(1);
    /// hard_reg.finish_cycle();
    /// assert_eq!(hard_reg.try_read().copied(), Some(i32::default()));
    /// ```
    pub fn write(&mut self, value: T) {
        if !self.hardwired {
            self.buf.write(Some(value))
        }
    }

    /// Marks the register as "busy", preventing any reads in subsequent clock cycles
    /// until a new value is written.
    ///
    /// This is primarily used for register renaming and scheduling - an instruction
    /// with a busy operand cannot be executed until the previous instruction that
    /// provides the operand computes the result.
    ///
    /// <div class="warning">
    ///
    /// It is important to note that when this method is called, the current value
    /// held by the register is dropped.
    ///
    /// </div>
    ///
    /// # Examples
    ///
    /// ```
    /// # use ferisc::core::reg::phys::PhysReg;
    /// # use ferisc::seq::Sequential;
    /// let mut reg = PhysReg::<i32>::new();
    /// assert_eq!(reg.try_read().copied(), Some(i32::default()));
    ///
    /// reg.mark_busy();
    /// reg.finish_cycle();
    /// assert_eq!(reg.try_read(), None);
    ///
    /// // Writing a new value marks the register as not busy
    /// reg.write(3);
    /// reg.finish_cycle();
    /// assert_eq!(reg.try_read().copied(), Some(3));
    /// ```
    pub fn mark_busy(&mut self) {
        if !self.hardwired {
            self.buf.write(None);
        }
    }
}

impl<T: Default> Default for PhysReg<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Sequential for PhysReg<T> {
    fn finish_cycle(&mut self) {
        self.buf.finish_cycle();
    }
}
