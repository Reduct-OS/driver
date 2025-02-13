use core::{fmt, ops::Deref};

use rstd::alloc::{borrow::ToOwned, string::String, sync::Arc, vec::Vec};
use spin::RwLock;

/// The raw SDT header struct, as defined by the ACPI specification.
#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct SdtHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

impl SdtHeader {
    pub fn signature(&self) -> SdtSignature {
        SdtSignature {
            signature: self.signature,
            oem_id: self.oem_id,
            oem_table_id: self.oem_table_id,
        }
    }
    pub fn length(&self) -> usize {
        self.length
            .try_into()
            .expect("expected usize to be at least 32 bits")
    }
}

unsafe impl plain::Plain for SdtHeader {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SdtSignature {
    pub signature: [u8; 4],
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
}

impl fmt::Display for SdtSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}",
            String::from_utf8_lossy(&self.signature),
            String::from_utf8_lossy(&self.oem_id),
            String::from_utf8_lossy(&self.oem_table_id)
        )
    }
}

struct PhysmapGuard {
    virt: *const u8,
    size: usize,
}

impl PhysmapGuard {
    fn map(page: usize, page_count: usize) -> Result<Self, TablePhysLoadError> {
        let size = page_count * 4096;
        rstd::mm::physmap(page, page, size);

        Ok(Self {
            virt: page as *const u8,
            size,
        })
    }
}

impl Deref for PhysmapGuard {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.virt as *const u8, self.size) }
    }
}

#[derive(Debug)]
pub enum InvalidSdtError {
    InvalidSize,
    BadChecksum,
}

impl fmt::Display for InvalidSdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => write!(f, "Invalid size of sdt"),
            Self::BadChecksum => write!(f, "Bad checksum for sdt"),
        }
    }
}

#[derive(Debug)]
pub enum TablePhysLoadError {
    Io,
    Validity(InvalidSdtError),
}

impl fmt::Display for TablePhysLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io => write!(f, "IO error"),
            Self::Validity(sdt_error) => write!(f, "{}", sdt_error),
        }
    }
}

#[derive(Clone)]
pub struct Sdt(Arc<[u8]>);

impl Sdt {
    pub fn new(slice: Arc<[u8]>) -> Result<Self, InvalidSdtError> {
        let header = match plain::from_bytes::<SdtHeader>(&slice) {
            Ok(header) => header,
            Err(plain::Error::TooShort) => return Err(InvalidSdtError::InvalidSize),
            Err(plain::Error::BadAlignment) => panic!(
                "plain::from_bytes failed due to alignment, but SdtHeader is #[repr(packed)]!"
            ),
        };

        if header.length() != slice.len() {
            return Err(InvalidSdtError::InvalidSize);
        }

        let checksum = slice
            .iter()
            .copied()
            .fold(0_u8, |current_sum, item| current_sum.wrapping_add(item));

        if checksum != 0 {
            return Err(InvalidSdtError::BadChecksum);
        }

        Ok(Self(slice))
    }

    pub fn load_from_physical(physaddr: usize) -> Result<Self, TablePhysLoadError> {
        let physaddr_start_page = physaddr / 4096 * 4096;
        let physaddr_page_offset = physaddr % 4096;

        // Begin by reading and validating the header first. The SDT header is always 36 bytes
        // long, and can thus span either one or two page table frames.
        let needs_extra_page = (4096 - physaddr_page_offset)
            .checked_sub(core::mem::size_of::<SdtHeader>())
            .is_none();
        let page_table_count = 1 + if needs_extra_page { 1 } else { 0 };

        let pages = PhysmapGuard::map(physaddr_start_page, page_table_count)?;
        assert!(pages.len() >= core::mem::size_of::<SdtHeader>());
        let sdt_mem = &pages[physaddr_page_offset..];

        let sdt = plain::from_bytes::<SdtHeader>(&sdt_mem[..core::mem::size_of::<SdtHeader>()])
            .expect("either alignment is wrong, or the length is too short, both of which are already checked for");

        let total_length = sdt.length();
        let base_length = core::cmp::min(total_length, sdt_mem.len());
        let extended_length = total_length - base_length;

        let mut loaded = sdt_mem[..base_length].to_owned();
        loaded.reserve(extended_length);

        const SIMULTANEOUS_PAGE_COUNT: usize = 4;

        let mut left = extended_length;
        let mut offset = physaddr_start_page + page_table_count * 4096;

        let length_per_iteration = 4096 * SIMULTANEOUS_PAGE_COUNT;

        while left > 0 {
            let to_copy = core::cmp::min(left, length_per_iteration);
            let additional_pages = PhysmapGuard::map(offset, to_copy.div_ceil(4096))?;

            loaded.extend(&additional_pages[..to_copy]);

            left -= to_copy;
            offset += to_copy;
        }
        assert_eq!(left, 0);

        Self::new(loaded.into()).map_err(|err| TablePhysLoadError::Validity(err))
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for Sdt {
    type Target = SdtHeader;

    fn deref(&self) -> &Self::Target {
        plain::from_bytes::<SdtHeader>(&self.0)
            .expect("expected already validated Sdt to be able to get its header")
    }
}

impl Sdt {
    pub fn data(&self) -> &[u8] {
        &self.0[core::mem::size_of::<SdtHeader>()..]
    }
}

impl fmt::Debug for Sdt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sdt")
            .field("header", &*self as &SdtHeader)
            .field("extra_len", &self.data().len())
            .finish()
    }
}

pub struct Dsdt(Sdt);
pub struct Ssdt(Sdt);

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
pub struct FadtStruct {
    pub header: SdtHeader,
    pub firmware_ctrl: u32,
    pub dsdt: u32,

    // field used in ACPI 1.0; no longer in use, for compatibility only
    reserved: u8,

    pub preferred_power_managament: u8,
    pub sci_interrupt: u16,
    pub smi_command_port: u32,
    pub acpi_enable: u8,
    pub acpi_disable: u8,
    pub s4_bios_req: u8,
    pub pstate_control: u8,
    pub pm1a_event_block: u32,
    pub pm1b_event_block: u32,
    pub pm1a_control_block: u32,
    pub pm1b_control_block: u32,
    pub pm2_control_block: u32,
    pub pm_timer_block: u32,
    pub gpe0_block: u32,
    pub gpe1_block: u32,
    pub pm1_event_length: u8,
    pub pm1_control_length: u8,
    pub pm2_control_length: u8,
    pub pm_timer_length: u8,
    pub gpe0_ength: u8,
    pub gpe1_length: u8,
    pub gpe1_base: u8,
    pub c_state_control: u8,
    pub worst_c2_latency: u16,
    pub worst_c3_latency: u16,
    pub flush_size: u16,
    pub flush_stride: u16,
    pub duty_offset: u8,
    pub duty_width: u8,
    pub day_alarm: u8,
    pub month_alarm: u8,
    pub century: u8,

    // reserved in ACPI 1.0; used since ACPI 2.0+
    pub boot_architecture_flags: u16,

    reserved2: u8,
    pub flags: u32,
}
unsafe impl plain::Plain for FadtStruct {}

#[repr(packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GenericAddressStructure {
    address_space: u8,
    bit_width: u8,
    bit_offset: u8,
    access_size: u8,
    address: u64,
}

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
pub struct FadtAcpi2Struct {
    // 12 byte structure; see below for details
    pub reset_reg: GenericAddressStructure,

    pub reset_value: u8,
    reserved3: [u8; 3],

    // 64bit pointers - Available on ACPI 2.0+
    pub x_firmware_control: u64,
    pub x_dsdt: u64,

    pub x_pm1a_event_block: GenericAddressStructure,
    pub x_pm1b_event_block: GenericAddressStructure,
    pub x_pm1a_control_block: GenericAddressStructure,
    pub x_pm1b_control_block: GenericAddressStructure,
    pub x_pm2_control_block: GenericAddressStructure,
    pub x_pm_timer_block: GenericAddressStructure,
    pub x_gpe0_block: GenericAddressStructure,
    pub x_gpe1_block: GenericAddressStructure,
}
unsafe impl plain::Plain for FadtAcpi2Struct {}

#[derive(Clone)]
pub struct Fadt(Sdt);

impl Fadt {
    pub fn acpi_2_struct(&self) -> Option<&FadtAcpi2Struct> {
        let bytes = &self.0.0[core::mem::size_of::<FadtStruct>()..];

        match plain::from_bytes::<FadtAcpi2Struct>(bytes) {
            Ok(fadt2) => Some(fadt2),
            Err(plain::Error::TooShort) => None,
            Err(plain::Error::BadAlignment) => unreachable!(
                "plain::from_bytes reported bad alignment, but FadtAcpi2Struct is #[repr(packed)]"
            ),
        }
    }
}

impl Deref for Fadt {
    type Target = FadtStruct;

    fn deref(&self) -> &Self::Target {
        plain::from_bytes::<FadtStruct>(&self.0.0)
            .expect("expected FADT struct to already be validated in Deref impl")
    }
}

impl Fadt {
    pub fn new(sdt: Sdt) -> Option<Fadt> {
        if sdt.signature != *b"FACP" || sdt.length() < core::mem::size_of::<Fadt>() {
            return None;
        }
        Some(Fadt(sdt))
    }

    pub fn init(context: &mut AcpiContext) {
        let fadt_sdt = context
            .take_single_sdt(*b"FACP")
            .expect("expected ACPI to always have a FADT");

        let fadt = match Fadt::new(fadt_sdt) {
            Some(fadt) => fadt,
            None => {
                println!("Failed to find FADT");
                return;
            }
        };

        let dsdt_ptr = match fadt.acpi_2_struct() {
            Some(fadt2) => usize::try_from(fadt2.x_dsdt).unwrap_or_else(|_| {
                usize::try_from(fadt.dsdt).expect("expected any given u32 to fit within usize")
            }),
            None => usize::try_from(fadt.dsdt).expect("expected any given u32 to fit within usize"),
        };

        println!("FACP at {:X}", { dsdt_ptr });

        let dsdt_sdt = match Sdt::load_from_physical(fadt.dsdt as usize) {
            Ok(dsdt) => dsdt,
            Err(error) => {
                println!("Failed to load DSDT: {}", error);
                return;
            }
        };

        context.fadt = Some(fadt.clone());
        context.dsdt = Some(Dsdt(dsdt_sdt.clone()));

        context.tables.push(dsdt_sdt);
    }
}

pub struct AcpiContext {
    tables: Vec<Sdt>,
    dsdt: Option<Dsdt>,
    fadt: Option<Fadt>,

    // TODO: The kernel ACPI code seemed to use load_table quite ubiquitously, however ACPI 5.1
    // states that DDBHandles can only be obtained when loading XSDT-pointed tables. So, we'll
    // generate an index only for those.
    sdt_order: RwLock<Vec<Option<SdtSignature>>>,

    pub next_ctx: RwLock<u64>,
}

impl AcpiContext {
    pub fn init(rxsdt_physaddrs: impl Iterator<Item = u64>) -> Self {
        let tables = rxsdt_physaddrs
            .map(|physaddr| {
                let physaddr: usize = physaddr
                    .try_into()
                    .expect("expected ACPI addresses to be compatible with the current word size");

                println!("TABLE AT {:#>08X}", physaddr);

                Sdt::load_from_physical(physaddr).expect("failed to load physical SDT")
            })
            .collect::<Vec<Sdt>>();

        let mut this = Self {
            tables,
            dsdt: None,
            fadt: None,

            next_ctx: RwLock::new(0),

            sdt_order: RwLock::new(Vec::new()),
        };

        for table in &this.tables {
            this.new_index(&table.signature());
        }

        Fadt::init(&mut this);
        //TODO (hangs on real hardware): Dmar::init(&this);

        this
    }

    pub fn dsdt(&self) -> Option<&Dsdt> {
        self.dsdt.as_ref()
    }
    pub fn ssdts(&self) -> impl Iterator<Item = Ssdt> + '_ {
        self.find_multiple_sdts(*b"SSDT")
            .map(|sdt| Ssdt(sdt.clone()))
    }
    fn find_single_sdt_pos(&self, signature: [u8; 4]) -> Option<usize> {
        let count = self
            .tables
            .iter()
            .filter(|sdt| sdt.signature == signature)
            .count();

        if count > 1 {
            println!(
                "Expected only a single SDT of signature `{}` ({:?}), but there were {}",
                String::from_utf8_lossy(&signature),
                signature,
                count
            );
        }

        self.tables
            .iter()
            .position(|sdt| sdt.signature == signature)
    }
    pub fn find_multiple_sdts<'a>(&'a self, signature: [u8; 4]) -> impl Iterator<Item = &'a Sdt> {
        self.tables
            .iter()
            .filter(move |sdt| sdt.signature == signature)
    }
    pub fn take_single_sdt(&self, signature: [u8; 4]) -> Option<Sdt> {
        self.find_single_sdt_pos(signature)
            .map(|pos| self.tables[pos].clone())
    }
    pub fn fadt(&self) -> Option<&Fadt> {
        self.fadt.as_ref()
    }
    pub fn sdt_from_signature(&self, signature: &SdtSignature) -> Option<&Sdt> {
        self.tables.iter().find(|sdt| {
            sdt.signature == signature.signature
                && sdt.oem_id == signature.oem_id
                && sdt.oem_table_id == signature.oem_table_id
        })
    }
    pub fn get_signature_from_index(&self, index: usize) -> Option<SdtSignature> {
        self.sdt_order.read().get(index).copied().flatten()
    }
    pub fn get_index_from_signature(&self, signature: &SdtSignature) -> Option<usize> {
        self.sdt_order
            .read()
            .iter()
            .rposition(|sig| sig.map_or(false, |sig| &sig == signature))
    }
    pub fn tables(&self) -> &[Sdt] {
        &self.tables
    }
    pub fn new_index(&self, signature: &SdtSignature) {
        self.sdt_order.write().push(Some(*signature));
    }
}

pub enum PossibleAmlTables {
    Dsdt(Dsdt),
    Ssdt(Ssdt),
}
impl PossibleAmlTables {
    pub fn try_new(inner: Sdt) -> Option<Self> {
        match &inner.signature {
            b"DSDT" => Some(Self::Dsdt(Dsdt(inner))),
            b"SSDT" => Some(Self::Ssdt(Ssdt(inner))),
            _ => None,
        }
    }
}
impl AmlContainingTable for PossibleAmlTables {
    fn aml(&self) -> &[u8] {
        match self {
            Self::Dsdt(dsdt) => dsdt.aml(),
            Self::Ssdt(ssdt) => ssdt.aml(),
        }
    }
    fn header(&self) -> &SdtHeader {
        match self {
            Self::Dsdt(dsdt) => dsdt.header(),
            Self::Ssdt(ssdt) => ssdt.header(),
        }
    }
}

pub trait AmlContainingTable {
    fn aml(&self) -> &[u8];
    fn header(&self) -> &SdtHeader;
}

impl<T> AmlContainingTable for &T
where
    T: AmlContainingTable,
{
    fn aml(&self) -> &[u8] {
        T::aml(*self)
    }
    fn header(&self) -> &SdtHeader {
        T::header(*self)
    }
}

impl AmlContainingTable for Dsdt {
    fn aml(&self) -> &[u8] {
        self.0.data()
    }
    fn header(&self) -> &SdtHeader {
        &*self.0
    }
}
impl AmlContainingTable for Ssdt {
    fn aml(&self) -> &[u8] {
        self.0.data()
    }
    fn header(&self) -> &SdtHeader {
        &*self.0
    }
}
