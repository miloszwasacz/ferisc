use super::{
    Addr, Mem,
    req::{LdReq, MemConn, ReqBuf, ReqData, ReqHandle, ReqState, StReq},
};
use crate::core::uop::Exception;
use crate::seq::Sequential;
use std::pin::Pin;

//TODO Proper DRAM simulation instead of simple emulation, with memory controller and different clocks

macro_rules! check_handle {
    ($handle:expr, $id:expr, $req_ty:ident) => {
        #[cfg(debug_assertions)]
        {
            assert_eq!($handle.id, $id);
            assert_eq!($handle.ty, super::ReqType::$req_ty);
        }
    };
}

const HANDLE_VALID_MSG: &str = "a caller should not have a handle if no request is being processed";

/// Dynamic Random Access Memory
#[derive(Debug, Clone)]
pub struct Dram {
    mem: Pin<Box<[u8]>>,
    ld_reqs: (Vec<ReqBuf<LdReq<Exception>>>, usize),
    st_reqs: (Vec<ReqBuf<StReq<Exception>>>, usize),
}

impl Dram {
    //TODO Better API for loading the binary
    pub fn new<'a>(
        size: usize,
        elf: elf::ElfBytes<'a, elf::endian::LittleEndian>,
        elf_data: &'a [u8],
    ) -> Self {
        let mut mem = vec![0; size].into_boxed_slice();
        //TODO Proper error handling
        for s in elf.segments().unwrap() {
            let offset = s.p_offset as usize;
            let addr = s.p_paddr as usize;
            let size = s.p_filesz.min(s.p_memsz) as usize;

            mem[addr..addr + size].copy_from_slice(&elf_data[offset..offset + size]);
        }

        Self {
            mem: Box::into_pin(mem),
            ld_reqs: (Vec::new(), 0),
            st_reqs: (Vec::new(), 0),
        }
    }

    #[cfg(debug_assertions)]
    fn id(&self) -> *const u8 {
        self.mem.as_ptr()
    }
}

impl Mem<LdReq<Exception>> for Dram {
    type Conn = MemConn;
    type Handle = ReqHandle;
    type Output = Result<ReqData, Exception>;

    fn register_conn(&mut self) -> Self::Conn {
        let conn = MemConn(self.ld_reqs.0.len());
        self.ld_reqs.0.push(Default::default());
        conn
    }

    fn req(&mut self, req: LdReq<Exception>, conn: Self::Conn) -> Self::Handle {
        self.ld_reqs.0[conn.0].write(req);
        ReqHandle {
            #[cfg(debug_assertions)]
            id: self.id(),
            #[cfg(debug_assertions)]
            ty: super::ReqType::Ld,
            conn,
        }
    }

    fn req_addr(&self, handle: &Self::Handle) -> Addr {
        check_handle!(handle, self.id(), Ld);
        self.ld_reqs.0[handle.conn.0]
            .read()
            .as_ref()
            .expect(HANDLE_VALID_MSG)
            .addr()
    }

    fn take_if_ready(&mut self, handle: Self::Handle) -> Result<Self::Output, Self::Handle> {
        check_handle!(handle, self.id(), Ld);
        let req = self.ld_reqs.0[handle.conn.0]
            .read()
            .expect(HANDLE_VALID_MSG);

        match req.state() {
            ReqState::Incomplete(_) => Err(handle),
            ReqState::Complete(data) => Ok(Ok(data.clone())),
            ReqState::Error(err) => Ok(Err(*err)),
        }
    }

    fn kill(&mut self, handle: Self::Handle) {
        check_handle!(handle, self.id(), Ld);
        self.ld_reqs.0[handle.conn.0].kill();
    }
}

impl Mem<StReq<Exception>> for Dram {
    type Conn = MemConn;
    type Handle = ReqHandle;
    type Output = Result<(), Exception>;

    fn register_conn(&mut self) -> Self::Conn {
        let conn = MemConn(self.st_reqs.0.len());
        self.st_reqs.0.push(Default::default());
        conn
    }

    fn req(&mut self, req: StReq<Exception>, conn: Self::Conn) -> Self::Handle {
        self.st_reqs.0[conn.0].write(req);
        ReqHandle {
            #[cfg(debug_assertions)]
            id: self.id(),
            #[cfg(debug_assertions)]
            ty: super::ReqType::St,
            conn,
        }
    }

    fn req_addr(&self, handle: &Self::Handle) -> Addr {
        check_handle!(handle, self.id(), St);
        self.st_reqs.0[handle.conn.0]
            .read()
            .as_ref()
            .expect(HANDLE_VALID_MSG)
            .addr()
    }

    fn take_if_ready(&mut self, handle: Self::Handle) -> Result<Self::Output, Self::Handle> {
        check_handle!(handle, self.id(), St);
        let req = self.st_reqs.0[handle.conn.0]
            .read()
            .expect(HANDLE_VALID_MSG);

        match req.state() {
            ReqState::Incomplete(_) => Err(handle),
            ReqState::Complete(()) => Ok(Ok(())),
            ReqState::Error(err) => Ok(Err(*err)),
        }
    }

    fn kill(&mut self, handle: Self::Handle) {
        check_handle!(handle, self.id(), St);
        self.st_reqs.0[handle.conn.0].kill();
    }
}

impl Sequential for Dram {
    fn finish_cycle(&mut self) {
        // The requests are executed in a round-robin manner across all connections
        macro_rules! finish_for_conns {
            ($conns:expr, $is_incomplete:expr, $complete:expr) => {{
                let mut iter = {
                    let (l, r) = $conns.0.split_at_mut($conns.1);
                    let offset = r.len();
                    let r = r.iter_mut().enumerate();
                    let l = l.iter_mut().enumerate().map(move |(i, e)| (i + offset, e));
                    r.chain(l)
                };
                // Find te first request that is incomplete
                let completed = iter
                    .find_map(|(i, req)| req.try_complete($is_incomplete, $complete).then_some(i))
                    .unwrap_or($conns.1);

                // Finish cycle on the remaining buffers
                for (_, req) in iter {
                    req.finish_cycle();
                }

                // Set the first-checked connection to the next one
                $conns.1 = completed.wrapping_add(1) % $conns.0.len();
            }};
        }

        finish_for_conns!(
            self.ld_reqs,
            |req| matches!(req.state(), ReqState::Incomplete(_)),
            |req| {
                //TODO Latency
                let data = self.mem[req.addr() as usize..][..req.size()].into();
                req.complete(data)
            }
        );
        finish_for_conns!(
            self.st_reqs,
            |req| matches!(req.state(), ReqState::Incomplete(_)),
            |req| {
                //TODO Latency
                let ReqState::Incomplete(data) = req.state() else {
                    panic!("request should be incomplete")
                };
                let mem = &mut self.mem[req.addr() as usize..][..req.size()];
                mem.copy_from_slice(data);
                req.complete()
            }
        );
    }
}
