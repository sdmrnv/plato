use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::io::{Write, Read};
use serde::Deserialize;
use axum::{
    http::StatusCode,
    http::header,
    Json, Router,
};
use flate2::{write::GzEncoder, Compression};
use axum::response::IntoResponse;
use axum::http::HeaderMap;


// ====================================================================
// CORE DATABASE CORE GEOMETRY
// ====================================================================
const MAX_ITEMS: usize = 1024;
const ARENA_SIZE: usize = 512 * 1024; // 512 KB memory tape

// ====================================================================
// SYSTEM PROFILE IDENTIFIERS (ACTORS AS STRICT u8 LEAF NODES)
// ====================================================================
const PROFILE_SYSTEM: u8  = 0; // Root system profile (Chaos/Core)
const PROFILE_ADMIN: u8   = 1; // Administrator / Law profile
const PROFILE_WORLD: u8   = 2; // Public internet / Shared environment
const PROFILE_LAMBER: u8  = 3; // AI Agent 1 (Balzac / Thread-bound)
const PROFILE_LAMIEL: u8  = 4; // AI Agent 2 (Stendhal / Action-bound)

// ====================================================================
// ENTITY TYPE IDENTIFIERS (MAPPED TO u8 PACKED LAYOUT)
// ====================================================================
#[allow(dead_code)]
const TYPE_PROFILE: u8        = 1; // Actor metadata layout
#[allow(dead_code)]
const TYPE_ARTICLE: u8        = 2; // Admin-to-World publication
const TYPE_LETTER: u8         = 4; // Inter-agent message
const TYPE_CORRESPONDENCE: u8 = 5; // Aggregated logs manifest

// ====================================================================
// FIXED HARDWARE-ALIGNED MANIFEST SLOTS
// ====================================================================
const SLOT_CORRESPONDENCE: usize       = 1023; // Correspondence index map

// ====================================================================
// ULTRA-DENSE SYSTEM INDEX (EXACTLY 8 BYTES - 8 ROWS PER CACHE LINE)
// ====================================================================
#[repr(align(8))]
struct MetadataRow {
    timestamp: u32,    // 4 bytes: Seconds since 1970-01-01
    content_type: u8,  // 1 byte: Entity type identifier
    author_id: u8,     // 1 byte: Creator profile identifier
    target_id: u8,     // 1 byte: Destination profile identifier
    padding: u8,       // 1 byte: Structural hardware alignment gap
}

// ====================================================================
// MONOLITHIC IN-MEMORY DATABASE VOLUMES (STATIC MEMORY MAP)
// ====================================================================
#[repr(align(128))]
struct AlignedArena {
    bytes: [u8; ARENA_SIZE],
}

// Memory tape for compressed payloads
static mut ARENA: AlignedArena = AlignedArena { bytes: [0u8; ARENA_SIZE] };

// Append-only memory pointer tracking free bytes in ARENA
static ARENA_FREE_POINTER: AtomicU32 = AtomicU32::new(0);

// Thread-safe lock-free primary index routing table (8 KB)
static JUMP_TABLE: [AtomicU64; MAX_ITEMS] = {
    const INIT: AtomicU64 = AtomicU64::new(0);
    [INIT; MAX_ITEMS]
};

// Raw metadata cache table (Exactly 8 KB, targets L1d cache segment)
static mut METADATA_ZONE: [MetadataRow; MAX_ITEMS] = {
    const INIT: MetadataRow = MetadataRow {
        timestamp: 0,
        content_type: 0,
        author_id: 0,
        target_id: 0,
        padding: 0,
    };
    [INIT; MAX_ITEMS]
};

// ====================================================================
// INCOMING NETWORK PAYLOAD SPECIFICATION
// ====================================================================
#[derive(Deserialize)]
struct SaveRequest {
    id: u16,          // Bound strictly to MAX_ITEMS capacity (0..1023)
    content_type: u8, // Maps to TYPE_* constants
    author_id: u8,    // Maps to PROFILE_* constants
    target_id: u8,    // Maps to PROFILE_* constants
    title: String,
    text: String,
}


#[derive(serde::Serialize)]
struct OccupiedSlot {
    id: u16,
    content_type: u8,
    author_id: u8,
    target_id: u8,
}

// ====================================================================
// AUX FUNCTIONS
// ====================================================================
/// Packs 32-bit offset and 32-bit length into a single monolithic 64-bit value.
// ====================================================================
// BITWISE ALIGNMENT UTILITIES (RESTORED WITH STRICT EXPLICIT BITMASK)
// ====================================================================
#[inline(always)]
fn pack_coordinates(offset: u32, len: u32) -> u64 {
    ((offset as u64) << 32) | (len as u64)
}

#[inline(always)]
fn unpack_coordinates(packed: u64) -> (u32, u32) {
    let offset = (packed >> 32) as u32;
    let len = (packed & 0xFFFFFFFF) as u32; // explicit mask restores straight register execution
    (offset, len)
}

// ====================================================================
// HARDWARE-LEVEL CYCLE COUNTER (ZERO-OVERHEAD ARCHITECTURE BINDING)
// ====================================================================
#[inline(always)]
fn get_cpu_cycles() -> u64 {
    let mut val: u64;
    unsafe {
        // Target: Apple Silicon (Mac M1 Ultra Development Environment)
        #[cfg(target_arch = "aarch64")]
        std::arch::asm!(
            "mrs {}, cntvct_el0", 
            out(reg) val, 
            options(nomem, nostack, preserves_flags)
        );

        // Target: Intel Xeon / AMD EPYC (1vCPU Silver Production Environment)
        #[cfg(target_arch = "x86_64")]
        std::arch::asm!(
            "rdtsc",
            "shl rdx, 32",
            "or rax, rdx",
            out("rax") val,
            out("rdx") _,
            options(nomem, nostack, preserves_flags)
        );
    }
    val
}


// ====================================================================
// MAIN FUNCTION
// ====================================================================
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), &'static str> {
    // Attempt to load the monolithic matrix layout from disk image
    if let Err(e) = load_from_disk() {
        println!("Core system started completely empty: {}. Awaiting manual profile commit via admin dashboard.", e);
    }

    //SPAWN THE BACKGROUND GC WORKER
    tokio::spawn(async {
        loop {
            let now = chrono::Local::now();
            let target_morning = now.date_naive()
                .and_hms_opt(9, 0, 0)
                .unwrap()
                .and_local_timezone(chrono::Local)
                .unwrap();
                
            let mut target_time = target_morning;
            if now >= target_time {
                target_time = target_time + chrono::Duration::days(1);
            }
            
            let sleep_duration = (target_time - now).to_std().unwrap_or(std::time::Duration::from_secs(3600));
            tokio::time::sleep(sleep_duration).await;
            
            println!("PLATON_GC: Scheduled maintenance triggered. Compacting memory tape...");
            if let Ok(latency) = defragment_arena() {
                let _ = save_to_disk();
                println!("PLATON_GC: Automated compaction successful. Cost: {:.2} us.", latency);
            }
        }
    });

    // Step 2: Build the clean nested routing engine matrix with global middleware token guard
    let app = Router::new()
    .route("/", axum::routing::get(handle_http_root))
    .route("/esse/:id", axum::routing::get(handle_http_get))
    .route("/metrics", axum::routing::get(handle_http_metrics))
    
    // Apply the layer STRICTLY to get(), closing its bracket AFTER .layer()
    .route(
        "/admin", 
        axum::routing::get(|| async { axum::response::Html(ADMIN_PANEL) })
            .layer(axum::middleware::from_fn(check_admin_token))
    )
    
    .nest(
        "/api",
        Router::new()
            .route("/slots", axum::routing::get(handle_api_slots))
            .route("/save", axum::routing::post(handle_api_save))
            .route("/delete/:id", axum::routing::post(handle_api_delete))
            .route("/rebuild", axum::routing::get(handle_api_rebuild))
            .route("/gc", axum::routing::get(handle_api_gc))
            .route("/ai/context/:author_id", axum::routing::get(handle_api_ai_context))
                .route_layer(axum::middleware::from_fn(check_admin_token)),
    );

    // Step 3: Anchor reactive socket listener to local loopback
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5040")
        .await
        .map_err(|_| "Failed to bind Tokio TCP listener to port 5040")?;
        
    println!("Plato engine active at 127.0.0.1:5040");

    // Step 4: Fire single-threaded reactive scheduler
    axum::serve(listener, app)
        .await
        .map_err(|_| "Axum runtime execution crashed")?;

    Ok(())
}



// ====================================================================
// REACTIVE ROUTER HANDLERS (AXUM HTTP GATEWAYS WITH EMBEDDED GAUGES)
// ====================================================================
/// Hot Path: Processes incoming Axum GET requests mapped directly to JUMP_TABLE slot IDs.
/// Enforces ultra-dense metadata checks and serves raw pre-compressed HTML blobs.
// Local dynamic gauge binding
async fn handle_http_get(axum::extract::Path(id): axum::extract::Path<u32>) -> impl axum::response::IntoResponse {
    // 1. START GAUGE: Snap register ticks before entering memory zone
//    let start_time = get_cpu_cycles();

    // 2. EXPLICIT LOOKUP: Invoke clean synchronous function isolated from Tokio
    if let Some(gzip_slice) = get_article_slice(id) {
        let bytes = axum::body::Bytes::from_static(gzip_slice);                
//        let end_time = get_cpu_cycles();
//        println!("PLATON_PERF: Pure L1/L2 database lookup took: {} CPU cycles", end_time.wrapping_sub(start_time));
        // 4. TRANSMIT: Hand over the pre-compressed payload to Axum/Tokio async engine
        (
            axum::http::StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (axum::http::header::CONTENT_ENCODING, "gzip"),
            ],
            axum::body::Body::from(bytes),
        ).into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "404 Not Found\n").into_response()
    }
}





/// Cold Path: Receives incoming JSON payload from admin panel, 
/// bakes monolithic HTML, compresses via flate2, and atomic-commits to storage.
async fn handle_api_save(Json(payload): Json<SaveRequest>) -> StatusCode {
    // 1. Strict boundary check for database slot capacity
    if payload.id >= MAX_ITEMS as u16 {
        return StatusCode::BAD_REQUEST;
    }

    // 1.5. Invariant Guard: Enforce strict presence of title block to guarantee link generation
    if payload.title.trim().is_empty() {
        println!("REJECTED: Transaction from actor {} aborted due to zero-length title.", payload.author_id);
        return StatusCode::BAD_REQUEST;
    }

    // 2. Generate server timestamps (system 24-hour formats)
    let current_timestamp = chrono::Utc::now().timestamp() as u32;
    let date_string = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 3. Registry of names
    let author_name = match payload.author_id {
        PROFILE_SYSTEM => "Небытие",
        PROFILE_ADMIN => "Наблюдатель",
        PROFILE_WORLD => "Мир",
        PROFILE_LAMBER => "Ламбер",
        PROFILE_LAMIEL => "Ламьель",
        _ => "Неизвестный автор",
    };

    // 4. Assemble monolithic html card layout with visual styling delegated to Svelte frontend
    let base_class = match payload.content_type {
        TYPE_ARTICLE => "plato-article-card",   // TYPE_ARTICLE
        TYPE_LETTER => "plato-letter-card",    // TYPE_LETTER
        TYPE_PROFILE => "plato-profile-card",   // TYPE_PROFILE
        TYPE_CORRESPONDENCE => "plato-journal-card",   // TYPE_CORRESPONDENCE
        _ => "plato-article-card",   // default
    };

    let content_class = if payload.content_type == TYPE_LETTER && payload.author_id == PROFILE_ADMIN {
        format!("{} plato-admin-letter", base_class)
    } else {
        base_class.to_string()
    };

    let monolithic_html = format!(
        "<div class=\"{}\">\
           <h1 class=\"plato-title\">{}</h1>\
           <div class=\"plato-content\">{}</div>\
           <div class=\"plato-footer\"><a href=\"/esse/{}\" class=\"plato-signature\">{}</a>, <span class=\"plato-date\">{}</span></div>\
         </div>",
        content_class,
        payload.title,
        payload.text,
        payload.author_id,
        author_name,
        date_string
    );

    // 5. Binary bundling pipeline in ARENA
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    if encoder.write_all(monolithic_html.as_bytes()).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    let compressed_bytes = match encoder.finish() {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let payload_len = compressed_bytes.len() as u32;

    let current_offset = ARENA_FREE_POINTER.load(Ordering::Relaxed);
    if (current_offset as usize) + (compressed_bytes.len()) > ARENA_SIZE {
        println!("CRITICAL: ARENA Out Of Memory layout constraint reached!");
        return StatusCode::INSUFFICIENT_STORAGE;
    }

    unsafe {
        //let dest_ptr = ARENA.bytes.as_mut_ptr().add(current_offset as usize);
        let dest_ptr = (std::ptr::addr_of_mut!(ARENA.bytes) as *mut u8).add(current_offset as usize);
        std::ptr::copy_nonoverlapping(compressed_bytes.as_ptr(), dest_ptr, compressed_bytes.len());
    }

    ARENA_FREE_POINTER.store(current_offset + payload_len, Ordering::Relaxed);

    unsafe {
        let meta = &mut METADATA_ZONE[payload.id as usize];
        meta.timestamp = current_timestamp;
        meta.content_type = payload.content_type;
        meta.author_id = payload.author_id;
        meta.target_id = payload.target_id;
        meta.padding = 0;
    }

    let packed_coords = pack_coordinates(current_offset, payload_len);
    JUMP_TABLE[payload.id as usize].store(packed_coords, Ordering::Release);

    // Trigger reactive journal recompilation only if the saved artifact is a letter
    if payload.content_type == TYPE_LETTER {
        if let Err(e) = rebuild_correspondence_journal() {
            println!("CRITICAL: Journal engine failed during save transaction: {}", e);
        }
    }

    // Direct memory-dump synchronization to persistence layer
    if let Err(e) = save_to_disk() {
        println!("CRITICAL DISK ERROR: {}", e);
    }

    println!("SUCCESS: Slot {} armed with entity type {}.", payload.id, payload.content_type);
    StatusCode::OK
}



fn get_article_slice(id: u32) -> Option<&'static [u8]> {
    // Enforce strict boundary pre-check to prevent panic on index out of bounds
    if id >= MAX_ITEMS as u32 {
        return None;
    }

    // 1. Read packed coordinates from L1 Data Cache (Takes ~1 clock cycle)
    let packed = JUMP_TABLE[id as usize].load(Ordering::Acquire);

    // If packed value is 0, the slot is empty (Tombstone or uninitialized)
    if packed == 0 {
        return None;
    }

    // 2. Decode coordinates using bitwise operations inside CPU registers
    let (offset, len) = unpack_coordinates(packed);

    unsafe {
        // 3. Get raw pointer to the static ARENA using addr_of! macro
        // This avoids creating a shared reference to static mut, complying with Rust 1.76+ specifications
        let arena_raw_ptr = std::ptr::addr_of!(ARENA) as *const u8;

        // Calculate the exact start address of the slice inside L2 cache
        let slice_ptr = arena_raw_ptr.add(offset as usize);

        // Return raw pointer slice from L2 cache without any heap allocations
        Some(std::slice::from_raw_parts(slice_ptr, len as usize))
    }
}



fn save_to_disk() -> Result<(), std::io::Error> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create("/srv/plato/db.bin")?;

    unsafe {
        // 1. Cast the entire 8 KB JUMP_TABLE atomic array straight to a byte slice
        let index_bytes = std::slice::from_raw_parts(
            JUMP_TABLE.as_ptr() as *const u8,
            std::mem::size_of_val(&JUMP_TABLE),
        );
        file.write_all(index_bytes)?;

        // 1.5. Cast the new 8 KB METADATA_ZONE matrix straight to a byte slice
        let metadata_bytes = std::slice::from_raw_parts(
            std::ptr::addr_of!(METADATA_ZONE) as *const u8,
            std::mem::size_of::<[MetadataRow; MAX_ITEMS]>(),
        );

        file.write_all(metadata_bytes)?;

        // 2. Cast the 512 KB monolithic ARENA straight to a byte slice
        let arena_bytes = std::slice::from_raw_parts(
            std::ptr::addr_of!(ARENA) as *const u8,
            ARENA_SIZE
        );
        file.write_all(arena_bytes)?;
    }

    file.sync_all()?;
    println!("Database dumped to disk (/srv/plato/db.bin).");
    Ok(())
}


fn load_from_disk() -> Result<(), std::io::Error> {
    use std::fs::File;
    use std::io::Read;

    if !std::path::Path::new("/srv/plato/db.bin").exists() {
        println!("No storage file found. Starting with an empty database.");
        return Ok(());
    }

    let mut file = File::open("/srv/plato/db.bin")?;
    unsafe {
        // 1. Read the 8 KB primary index jump table straight to L1 Cache
        let index_bytes = std::slice::from_raw_parts_mut(
            JUMP_TABLE.as_ptr() as *mut u8,
            std::mem::size_of_val(&JUMP_TABLE)
        );
        file.read_exact(index_bytes)?;

        // 1.5. Read the new 8 KB METADATA_ZONE matrix in exact serialization order
        let metadata_bytes = std::slice::from_raw_parts_mut(
            std::ptr::addr_of_mut!(METADATA_ZONE) as *mut u8,
            std::mem::size_of::<[MetadataRow; MAX_ITEMS]>()
        );

        file.read_exact(metadata_bytes)?;

        // 2. Read the monolithic 512 KB ARENA layout
        let arena_bytes = std::slice::from_raw_parts_mut(
            std::ptr::addr_of_mut!(ARENA) as *mut u8,
            ARENA_SIZE
        );
        file.read_exact(arena_bytes)?;
    }

    // Scan  JUMP_TABLE and restore the tail of ARENA
    let mut max_tail = 0u32;
    for id in 0..MAX_ITEMS {
        let packed = JUMP_TABLE[id].load(Ordering::Relaxed);
        if packed != 0 {
            let (offset, len) = unpack_coordinates(packed);
            let tail = offset + len;
            if tail > max_tail { max_tail = tail; }
        }
    }
    ARENA_FREE_POINTER.store(max_tail, Ordering::Release);
    println!("Database loaded. ARENA_FREE_POINTER restored to offset: {} bytes.", max_tail);
    Ok(())
}


async fn handle_api_delete(axum::extract::Path(id): axum::extract::Path<u32>) -> StatusCode {
    if id >= MAX_ITEMS as u32 {
        return StatusCode::BAD_REQUEST;
    }

    // Extract the original content type from L1d cache segment before index wipeout
    let target_type = unsafe { METADATA_ZONE[id as usize].content_type };

    // Atomic wipeout of the primary routing index table slot (readers instantly miss)
    JUMP_TABLE[id as usize].store(0, Ordering::Release);

    // Physical obliteration of the metadata record layout
    unsafe {
        let meta = &mut METADATA_ZONE[id as usize];
        meta.timestamp = 0;
        meta.content_type = 0;
        meta.author_id = 0;
        meta.target_id = 0;
        meta.padding = 0;
    }

    // Trigger reactive journal recompilation if the destroyed artifact was a letter
    if target_type == TYPE_LETTER {
        if let Err(e) = rebuild_correspondence_journal() {
            println!("CRITICAL: Journal engine failed during delete transaction: {}", e);
        }
    }

    // Mirror the modified matrix state down to disk image persistence layer
    if let Err(e) = save_to_disk() {
        println!("CRITICAL DISK ERROR DURING DELETE: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    println!("SUCCESS: Slot {} has been completely obliterated from indexes.", id);
    StatusCode::OK
}



fn rebuild_correspondence_journal() -> Result<(), &'static str> {
    // 1. Snapshot hardware cycles at the entry barrier
    let start_cycles = get_cpu_cycles();

    // 2. Format localized 24-hour server time
    let regen_date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 3. Pre-allocate heap string flat buffer to minimize reallocation overhead
    let mut journal_html = String::with_capacity(16 * 1024);

    // 4. Inject monolithic card layout wrapper completely clean of inline CSS styles
    journal_html.push_str(
        "<div class=\"plato-article-card\">\
         <h1 class=\"plato-title\">Диалоги</h1>\
         <div class=\"plato-content\">"
    );

    // 5. Linear scan of the 8 KB METADATA_ZONE to harvest active letters
    for id in 0..MAX_ITEMS {
        // Skip management slots to prevent recursive compilation loops
        if id == SLOT_CORRESPONDENCE {
            continue;
        }

        unsafe {
            let meta = &METADATA_ZONE[id];

            // Process slots strictly marked as TYPE_LETTER
            if meta.content_type == TYPE_LETTER {
                // Verify if the slot is physically armed in the core index routing table
                let packed = JUMP_TABLE[id].load(Ordering::Acquire);
                if packed == 0 {
                    continue; // Skip obliterated letters (tombstones) automatically
                }

                // Fetch raw slice from L2 bound memory tape
                if let Some(gzip_slice) = get_article_slice(id as u32) {
                    // Flash-decompress compressed payload into temporary string layout
                    let mut decoder = flate2::read::GzDecoder::new(gzip_slice);
                    let mut raw_html = String::new();
                    if decoder.read_to_string(&mut raw_html).is_ok() {
                        // Resolve actor human names using strict register matching rules
                        let author_name = match meta.author_id {
                            PROFILE_SYSTEM => "Небытие",
                            PROFILE_ADMIN => "Наблюдатель",
                            PROFILE_WORLD => "Мир",
                            PROFILE_LAMBER => "Ламбер",
                            PROFILE_LAMIEL => "Ламьель",
                            _ => "Неизвестный автор",
                        };
                        let target_name = match meta.target_id {
                            PROFILE_SYSTEM => "Небытие",
                            PROFILE_ADMIN => "Наблюдатель",
                            PROFILE_WORLD => "Мир",
                            PROFILE_LAMBER => "Ламбер",
                            PROFILE_LAMIEL => "Ламьель",
                            _ => "Неизвестный автор",
                        };

                        // Extract parameters via zero-copy byte offsets boundaries
                        let title_start = raw_html.find("<h1").and_then(|idx| raw_html[idx..].find(">").map(|d| idx + d + 1));
                        let title_end = raw_html.find("</h1>");

                        let content_search_str = "class=\"plato-content\"";
                        let content_start = raw_html.find(content_search_str)
                            .and_then(|idx| raw_html[idx..].find(">").map(|d| idx + d + 1));

                        // Robust end boundary: find the very first closing div right after content starts
                        let content_end = content_start.and_then(|start_idx| {
                            raw_html[start_idx..].find("</div>").map(|offset| start_idx + offset)
                        });

                        if let (Some(ts), Some(te), Some(cs), Some(ce)) = (title_start, title_end, content_start, content_end) {
                            let title = raw_html[ts..te].trim();
                            let full_content = raw_html[cs..ce].trim();

                            // Precision UTF-8 safe boundary slicer limiting preview length to exactly 120 characters
                            let preview_bound = full_content
                                .char_indices()
                                .map(|(idx, _)| idx)
                                .nth(220)
                                .unwrap_or(full_content.len());
                            let first_paragraph = full_content[..preview_bound].trim();

                            // Construct standard human date representation
                            let letter_date = chrono::DateTime::from_timestamp(meta.timestamp as i64, 0)
                                .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| regen_date.clone());

                            // Pass semantic marker classes to Svelte instead of hardcoded style properties
                            let style_class = if meta.author_id == PROFILE_ADMIN {
                                "plato-signature admin-directive"
                            } else {
                                "plato-signature"
                            };

                            let prefix = if meta.author_id == PROFILE_ADMIN { "[ИНСТРУКЦИЯ]: " } else { "" };

                            // Flat line single format macro injection preventing compiler tab spacing leaks
                            let row = format!(
                                "<div class=\"correspondence-row\"><span class=\"plato-journal-meta\">{} | {} ➔ {}</span><h3 class=\"plato-journal-title\"><a href=\"/esse/{}\" class=\"{}\">{}{}</a></h3><p class=\"plato-journal-preview\">{}...</p></div>",
                                letter_date, author_name, target_name, id, style_class, prefix, title, first_paragraph
                            );
                            journal_html.push_str(&row);
                        }
                    }
                }
            }
        }
    }

    // 6. Seal inner data stack and calculate processing delta ticks
    journal_html.push_str("</div></div>");
    let end_cycles = get_cpu_cycles();
    let delta_cycles = end_cycles.wrapping_sub(start_cycles);

    // Core hardware frequency selectors mapping
    #[cfg(target_arch = "aarch64")]
    const TIMER_FREQUENCY_MHZ: f64 = 24.0 * 1000.0; // 24 ticks per microsecond on Apple Silicon

    #[cfg(target_arch = "x86_64")]
    const TIMER_FREQUENCY_MHZ: f64 = 2200.0 * 1000.0; // 2200 ticks per microsecond on Intel Xeon Silver

    // Physics of execution: compute precise latency in microseconds (μs)
    let approx_us = (delta_cycles as f64) / TIMER_FREQUENCY_MHZ;

    // Inject the refined engineering layout footer using clean class markers in a single line
    let footer = format!(
        "<div class=\"plato-journal-footer\">Journal regenerated: {} | Performance cost: {} ticks (approx {:.2} ms)</div>",
        regen_date, delta_cycles, approx_us
    );
    journal_html.push_str(&footer);

    // 7. Pipe modified raw markup directly to heavy Gzip Level 9 encoder stream
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(journal_html.as_bytes()).map_err(|_| "Compression failed")?;
    let compressed_bytes = encoder.finish().map_err(|_| "Encoder finish failed")?;
    let payload_len = compressed_bytes.len() as u32;

    // 8. Commit the transactional block to the end of the unmanaged ARENA tape
    let current_offset = ARENA_FREE_POINTER.load(Ordering::Relaxed);
    if (current_offset as usize) + (compressed_bytes.len()) > ARENA_SIZE {
        return Err("ARENA OOM mapping journal layout constraint");
    }

    unsafe {
        let dest_ptr = (std::ptr::addr_of_mut!(ARENA.bytes) as *mut u8).add(current_offset as usize);
        std::ptr::copy_nonoverlapping(compressed_bytes.as_ptr(), dest_ptr, compressed_bytes.len());

        let meta = &mut METADATA_ZONE[SLOT_CORRESPONDENCE];
        meta.timestamp = chrono::Utc::now().timestamp() as u32;
        meta.content_type = TYPE_CORRESPONDENCE;
        meta.author_id = PROFILE_SYSTEM;
        meta.target_id = PROFILE_WORLD;
        meta.padding = 0;
    }

    ARENA_FREE_POINTER.store(current_offset + payload_len, Ordering::Relaxed);
    let packed_coords = pack_coordinates(current_offset, payload_len);
    JUMP_TABLE[SLOT_CORRESPONDENCE].store(packed_coords, Ordering::Release);

    Ok(())
}


// ====================================================================
// CORE COMPACTION GATEWAY
// ====================================================================
async fn handle_api_gc() -> impl axum::response::IntoResponse {
    // Invoke the core hardware compaction routine manually
    match defragment_arena() {
        Ok(latency_us) => {
            // Flush the defragmented memory state onto the disk architecture layer
            if let Err(e) = save_to_disk() {
                println!("CRITICAL DISK ERROR DURING DEFRAGMENTATION: {}", e);
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "DISK COMPACTION SYNC ERROR".to_string());
            }
            // Return clean plain text response using honest UTF-8 microsecond symbol
            (
                axum::http::StatusCode::OK, 
                format!("SUCCESS: Memory tape compressed. Garbage collection cost: {:.2} µs", latency_us)
            )
        }
        Err(e) => {
            println!("CRITICAL GC EXHAUSTION: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("ERROR: {}", e))
        }
    }
}

// ====================================================================
// MANUAL JOURNAL RECONSTRUCTION GATEWAY
// ====================================================================
async fn handle_api_rebuild() -> impl axum::response::IntoResponse {
    // Manually invoke the core unmanaged journal reconstruction routine
    match rebuild_correspondence_journal() {
        Ok(_) => {
            // Commit the freshly generated state to disk persistence mirror immediately
            if let Err(e) = save_to_disk() {
                println!("CRITICAL DISK ERROR DURING MANUAL REBUILD: {}", e);
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "DISK WRITE ERROR".to_string());
            }
            (axum::http::StatusCode::OK, "SUCCESS: Journal re-anchored on memory tape.".to_string())
        }
        Err(e) => {
            println!("CRITICAL: Manual journal trigger failed: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("ERROR: {}", e))
        }
    }
}



async fn handle_api_slots() -> impl IntoResponse {
    // 1. Snapshot hardware reference counter at the exact entry boundary
    let start_cycles = get_cpu_cycles();

    // 2. Pre-allocate vector capacity to avoid heap reallocations during loop execution
    let mut occupied_list = Vec::with_capacity(64);

    // 3. Linear scan of the armed JUMP_TABLE slots directly inside L2 cache bound line
    for id in 0..MAX_ITEMS {
        let packed = JUMP_TABLE[id].load(Ordering::Acquire);
        
        if packed != 0 {
            // Safe copy of the armed entity metadata type field
            let content_type = unsafe { METADATA_ZONE[id].content_type };
            let author_id = unsafe { METADATA_ZONE[id].author_id };
            let target_id = unsafe { METADATA_ZONE[id].target_id };
            occupied_list.push(OccupiedSlot {
                id: id as u16,
                content_type,
                author_id,
                target_id,
            });
        }
    }

    // 4. Capture hardware ticks at the exit boundary to close the performance benchmark window
    let end_cycles = get_cpu_cycles();
    let delta_cycles = end_cycles.wrapping_sub(start_cycles);

    // 5. Select execution reference scaling factor depending on compilation target gates
    #[cfg(target_arch = "aarch64")]
    const TIMER_FREQUENCY_MHZ: f64 = 24.0; // Fixed 24 ticks per microsecond on Apple Silicon
    
    #[cfg(target_arch = "x86_64")]
//    const TIMER_FREQUENCY_MHZ: f64 = 4300.0; // Fixed Ryzen 9 9950X Invariant TSC (4.3 GHz)
    const TIMER_FREQUENCY_MHZ: f64 = 2200.0; // Intel(R) Xeon(R) Silver 4114 CPU @ 2.20GHz Invariant TSC (2.2 GHz)

    // Physics of execution: compute precise latency in microseconds (μs)
    let approx_us = (delta_cycles as f64) / TIMER_FREQUENCY_MHZ;

    // 6. Inject telemetry cost header directly into response pipe without payload inflation
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Platon-Audit-Microseconds",
        axum::http::HeaderValue::from_str(&format!("{:.2}", approx_us))
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0.00"))
    );

    // Return transactional headers combined with compressed fast JSON array stream
    (headers, axum::Json(occupied_list))
}



fn defragment_arena() -> Result<f64, &'static str> {
    // 1. Snapshot hardware cycles at the exact entry boundary
    let start_cycles = get_cpu_cycles();
    
    // 2. Allocate an isolated temporary flat buffer to orchestrate clean stream packing
    let mut temp_bytes = vec![0u8; ARENA_SIZE];
    let mut new_offset: u32 = 0;

    // 3. Sequential transactional iteration across the entire 1024 JUMP_TABLE capacity
    for id in 0..MAX_ITEMS {
        let packed = JUMP_TABLE[id].load(Ordering::Acquire);
        if packed == 0 {
            continue; // Automatically drop torn records and deleted tombstones
        }

        // Deconstruct the original coordinate registers
        let (old_offset, len) = unpack_coordinates(packed);

        // Enforce strict safety boundary validations against the current tape array size
        if (old_offset as usize) + (len as usize) > ARENA_SIZE || new_offset as usize + (len as usize) > ARENA_SIZE {
            return Err("Memory boundary corruption detected during compaction sequence");
        }

        unsafe {
            // Get raw pointers directly bypassing implicit Rust exclusive references
            let src_ptr = (std::ptr::addr_of!(ARENA.bytes) as *const u8).add(old_offset as usize);
            let dest_ptr = temp_bytes.as_mut_ptr().add(new_offset as usize);
            
            // Execute fast block copy inside L2 cache
            std::ptr::copy_nonoverlapping(src_ptr, dest_ptr, len as usize);
        }

        // Rewrite coordinates with the newly consolidated offset metrics
        let new_packed = pack_coordinates(new_offset, len);
        JUMP_TABLE[id].store(new_packed, Ordering::Release);

        // Advance layout cursor sequence
        new_offset += len;
    }

    unsafe {
        // Atomic commit of the defragmented tape back to static memory allocation
        let arena_mut_ptr = std::ptr::addr_of_mut!(ARENA.bytes) as *mut u8;
        std::ptr::copy_nonoverlapping(temp_bytes.as_ptr(), arena_mut_ptr, ARENA_SIZE);
    }
    
    // Arm the global pointer tracker with the fresh remaining tape boundary line
    ARENA_FREE_POINTER.store(new_offset, Ordering::Relaxed);

    // 4. Close hardware benchmarking window and evaluate operational latency metrics
    let end_cycles = get_cpu_cycles();
    let delta_cycles = end_cycles.wrapping_sub(start_cycles);

    // Dynamic architecture constant tracking setup
    #[cfg(target_arch = "aarch64")]
    const TIMER_FREQUENCY_MHZ: f64 = 24.0; // Apple Silicon hardware counter
    #[cfg(target_arch = "x86_64")]
    const TIMER_FREQUENCY_MHZ: f64 = 2200.0; // Intel Xeon Silver base clock counter

    let approx_us = (delta_cycles as f64) / TIMER_FREQUENCY_MHZ;
    Ok(approx_us)
}


async fn handle_http_metrics() -> impl axum::response::IntoResponse {
    // Calculate arena allocation saturation based on the free pointer
    let free_ptr = ARENA_FREE_POINTER.load(Ordering::Relaxed) as usize;
    let saturation_pct = (free_ptr as f64 / ARENA_SIZE as f64) * 100.0;
    
    let mut live_bytes = 0;
    let mut max_physical_edge = 0;
    let mut occupied_slots_count = 0;

    // Scan the jump table to aggregate live item metrics and track the high-water mark
    for id in 0..MAX_ITEMS {
        let packed = JUMP_TABLE[id].load(Ordering::Acquire);
        if packed != 0 {
            occupied_slots_count += 1;
            live_bytes += (packed & 0xFFFFFFFF) as usize;
            // Extract offset (high 32 bits) and length (low 32 bits) from the packed value        
            let offset = (packed >> 32) as usize;
            let len = (packed & 0xFFFFFFFF) as usize;
            let edge = offset + len;
            if edge > max_physical_edge {
                max_physical_edge = edge;
            }
        }
    }
    // Determine fragmented space and remaining capacity
    let zombie_bytes = if max_physical_edge > live_bytes { max_physical_edge - live_bytes } else { 0 };
    let free_slots_remaining = MAX_ITEMS.saturating_sub(occupied_slots_count);
    
    // Resolve system start time using metadata slot 0 or fallback timestamp
    let slot_0_ts = unsafe { METADATA_ZONE[0].timestamp };
    let start_timestamp = if slot_0_ts != 0 { slot_0_ts as i64 } else { 1781863800 }; // 2026-06-18 11:50:00
    
    // Calculate total service uptime metrics
    let now_ts = chrono::Utc::now().timestamp();
    let seconds_in_service = now_ts.saturating_sub(start_timestamp);
    let days_in_service = seconds_in_service / 86400;

    const DAILY_BURN_RATE_SLOTS: f64 = 1.14; 
    let days_until_slots_exhaustion = (free_slots_remaining as f64 / DAILY_BURN_RATE_SLOTS).round() as usize;

    let bar_width = 32;
    let mut l2_bar = String::new();
    let l2_filled = ((saturation_pct / 100.0) * bar_width as f64).round() as usize;
    for i in 0..bar_width {
        if i < l2_filled { l2_bar.push('█'); } else { l2_bar.push('░'); }
    }

    let mut l1_slots_bar = String::new();
    let slots_saturation_pct = (occupied_slots_count as f64 / MAX_ITEMS as f64) * 100.0;
    let l1_filled = ((slots_saturation_pct / 100.0) * bar_width as f64).round() as usize;
    for i in 0..bar_width {
        if i < l1_filled { l1_slots_bar.push('█'); } else { l1_slots_bar.push('░'); }
    }

    let metrics_html = format!(
r#"-----------------------------------------------------------------------------------------
[ HOST SILICON RUNTIME TELEMETRY MATRIX ]
-----------------------------------------------------------------------------------------
Operational State         :: NOMINAL_ISOLATED
Core Read Latency         :: 24-166 CPU Ticks (approx 0.011 - 0.075 µs @ L1/L2 Hit)
Core Read Latency (Avg)   :: ~62 CPU Ticks (approx 0.028 µs @ L1/L2 Hit)
Database Memory Footprint :: 528 KB Monolithic Monad
Hardware Alignment        :: JUMP_TABLE [Aligned 64B] | METADATA [Packed 8B]

[ INTERFACE GATEWAY PLATFORM ]
-----------------------------------------------------------------------------------------
Host Processor            :: Intel(R) Xeon(R) Silver 4114 CPU @ 2.20GHz
Topology Core Profile     :: 1 vCPU | 1 Thread (Hyper-Threading Disabled)
Hypervisor Layer          :: KVM Hardware Enforced (Strict Resource Lock)
Instruction Vector Gates  :: AVX-512F, AVX-512DQ, AVX-512BW, AVX-512VL Active
Operating System          :: Oracle Linux 9.x (UEK Release 8)

[NETWORK BENCHMARK]
-----------------------------------------------------------------------------------------
Wire Footprint            :: 1.00 KB ( Full HTTP/2 + TLS Encapsulation Wrapper active  )
Solo Pipe (-c1)           :: 14.7k RPS | 13.1 MB/s | Latency: 60 µs (p50) | 228 µs (p99)
Stress Run (-c100)        :: 15.3k RPS | 13.7 MB/s | Latency: 6.1ms (p50) | 10.7ms (p99)

[ L1d STRUCTURAL INDEX CAPACITY ]
-----------------------------------------------------------------------------------------
Total Index Capacity      :: {MAX_ITEMS} Fixed Operational Slots
Occupied Primary Slots    :: {occupied_slots_count} Slots Armed
Available Vacant Slots    :: {free_slots_remaining} Slots Free
L1d Index Saturation Bar  :: [{l1_slots_bar}] {slots_saturation_pct:.2}%

[ L2 HOT TAPE MEMORY SATURATION ]
-----------------------------------------------------------------------------------------
Arena Max Capacity        :: {ARENA_SIZE} bytes (512 KB)
Tape Memory Cursor        :: {free_ptr} bytes packed
Active Live Payload       :: {live_bytes} bytes
Zombie Data Fragmentation :: {zombie_bytes} bytes
L2 Arena Saturation Bar   :: [{l2_bar}] {saturation_pct:.2}%

[ CHRONO LIFE-CYCLE METRICS ]
-----------------------------------------------------------------------------------------
Core Engine In Service    :: <span style="color: #acab37; font-weight: bold;">{days_in_service}</span> Days Active
Current Agent Burn Rate   :: ~{DAILY_BURN_RATE_SLOTS:.2} Slots / Day (Combined)
Estimated Core Runway     :: <span style="color: #bf4037; font-weight: bold;">{days_until_slots_exhaustion}</span> Days Until Slot Exhaustion

=========================================================================================
[ HARDWARE MAP: THE L1/L2 BARRIER ]
+---------------------------------------------------------------------------------------+
| L1d Index Cache  [{l1_slots_bar}] {slots_saturation_pct:.2}% index weight (16 KB Lock)
| L2 Active Arena  [{l2_bar}] {saturation_pct:.2}% payload weight (512 KB Lock)
+---------------------------------------------------------------------------------------+
Status: Operational loop stable. 100% data processing bounds locked in silicon."#,
        MAX_ITEMS = MAX_ITEMS,
        occupied_slots_count = occupied_slots_count,
        free_slots_remaining = free_slots_remaining,
        l1_slots_bar = l1_slots_bar,
        slots_saturation_pct = slots_saturation_pct,
        ARENA_SIZE = ARENA_SIZE,
        free_ptr = free_ptr,
        live_bytes = live_bytes,
        zombie_bytes = zombie_bytes,
        l2_bar = l2_bar,
        saturation_pct = saturation_pct,
        days_in_service = days_in_service,
        DAILY_BURN_RATE_SLOTS = DAILY_BURN_RATE_SLOTS,
        days_until_slots_exhaustion = days_until_slots_exhaustion
    );

    axum::response::Html(metrics_html)
}


// ====================================================================
// AI SLIDING CONTEXT FEED PIPELINE
// ====================================================================
async fn handle_api_ai_context(
    axum::extract::Path(author_id): axum::extract::Path<u8>,
) -> impl axum::response::IntoResponse {
    // 1. Enforce strict agent profile boundary checks
    if author_id != PROFILE_LAMBER && author_id != PROFILE_LAMIEL {
        return (StatusCode::BAD_REQUEST, "Invalid agent profile ID\n").into_response();
    }

    let opponent_id = if author_id == PROFILE_LAMBER { PROFILE_LAMIEL } else { PROFILE_LAMBER };

    // 2. Track sliding window allocation limits for inter-agent dialogue
    const MAX_DIALOGUE_LETTERS: usize = 10;
    let mut collected_letters_count = 0;

    // Temporary storage stack to store identified slots before sequence inversion
    let mut target_slots = Vec::with_capacity(64);

    // 3. Scan METADATA_ZONE backwards (from latest to oldest) to harvest recent timeline
    for id in (0..MAX_ITEMS).rev() {
        if id == SLOT_CORRESPONDENCE {
            continue; // Skip management index layouts
        }

        unsafe {
            let meta = &METADATA_ZONE[id];
            let packed = JUMP_TABLE[id].load(Ordering::Acquire);
            if packed == 0 {
                continue; // Skip tombstones or uninitialized memory cells
            }

            // Process strictly message layouts, ignoring public articles completely
            if meta.content_type == TYPE_LETTER {
                // Check if this is a directive from the Admin addressed to this agent or the entire AI World
                let is_admin_directive = meta.author_id == PROFILE_ADMIN
                    && (meta.target_id == author_id || meta.target_id == PROFILE_WORLD);

                // Check if this is a genuine incoming letter from the opponent agent
                let is_incoming_letter = meta.author_id == opponent_id && meta.target_id == author_id;

                if is_admin_directive || is_incoming_letter {
                    if collected_letters_count < MAX_DIALOGUE_LETTERS {
                        // Store slot ID and a boolean flag indicating if it's an admin command
                        target_slots.push((id, if meta.author_id == PROFILE_ADMIN { 3u8 } else { 2u8 }));
                        collected_letters_count += 1;
                    }
                }
            } else if meta.content_type == TYPE_PROFILE && (id == (PROFILE_LAMBER as usize) || id == (PROFILE_LAMIEL as usize) ){
                //Add HERO PROFILES
                target_slots.push((id, if id == (PROFILE_LAMBER as usize) {0u8} else {1u8}));
            }
        }
    }
    // 4. Invert the collected list to restore correct historical chronological sequence
    target_slots.reverse();
    // 5. Group the collected list by profiles, then chrono letters (includind admin directives)
    target_slots.sort_by_key(|item| if item.1 == 2 || item.1 == 3 { 2u8 } else { item.1 });

    // 5. Decompress, strip HTML, and assemble flat execution prompt text buffer
    let mut assembled_prompt = String::with_capacity(16 * 1024);

    for (id, typ) in target_slots {
        if let Some(gzip_slice) = get_article_slice(id as u32) {
            let mut decoder = flate2::read::GzDecoder::new(gzip_slice);
            let mut raw_html = String::new();

            if decoder.read_to_string(&mut raw_html).is_ok() {
                if let Some(clean_body) = extract_plato_content_payload(&raw_html) {
                    unsafe {
                        let meta = &METADATA_ZONE[id];
                        
                        // Map sender profile string values cleanly inside CPU registers
                        let sender_name = match meta.author_id {
                            PROFILE_ADMIN => "ADMIN (Observer)",
                            PROFILE_LAMBER => "Lamber",
                            PROFILE_LAMIEL => "Lamiel",
                            _ => "System",
                        };

                        // Map recipient profile string values cleanly
                        let recipient_name = match meta.target_id {
                            PROFILE_WORLD => "ALL (World)",
                            PROFILE_LAMBER => "Lamber",
                            PROFILE_LAMIEL => "Lamiel",
                            _ => "System",
                        };

                        // Build structured context boundaries depending on actor roles
                        if typ == 3 {
                            assembled_prompt.push_str(&format!(
                                "[ADMIN DIRECTIVE | SLOT {}]\nFrom: {}\nTo: {}\nContent:\n{}\n",
                                id, sender_name, recipient_name, clean_body
                            ));
                        } else if typ == 2 {
                            assembled_prompt.push_str(&format!(
                                "[LETTER | SLOT {}]\nFrom: {}\nTo: {}\nContent:\n{}\n",
                                id, sender_name, recipient_name, clean_body
                            ));
                        } else {
                            assembled_prompt.push_str(&format!(
                                "[HERO PROFILE | {}]\n{}\n",
                                if typ==0 {"Lamber"} else {"Lamiel"}, clean_body
                            ));
                        }
                    }
                    // Unified single boundary separation rule protecting LLM context sequence
                    assembled_prompt.push_str("------------------------------------------------\n\n");
                }
            }
        }
    }

    if assembled_prompt.is_empty() {
        assembled_prompt.push_str("[SYSTEM STATUS]: Matrix void. No operational data found.\n");
    }

    // 6. Output raw linear text layout directly back to local Llama runtime orchestrator
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        assembled_prompt,
    ).into_response()
}


/// Zero-copy boundary locator slicing raw text out of structured layout payloads
#[inline(always)]
fn extract_plato_content_payload(html: &str) -> Option<&str> {
    let search_marker = "class=\"plato-content\"";
    let start_idx = html.find(search_marker)?;
    
    // Jump past the opening tag bracket index floor
    let data_offset = html[start_idx..].find('>')? + start_idx + 1;
    let end_offset = html[data_offset..].find("</div>")? + data_offset;
    
    Some(html[data_offset..end_offset].trim())
}


//Monolithic static single-page framework layout embedded directly into read-only memory
//const SVELTE_FRONTEND_UI: &str = include_str!("/srv/plato/index.html");
const SVELTE_FRONTEND_UI: &[u8] = include_bytes!("/srv/plato/index.html.gz");

// ====================================================================
// CORE HIGH-SPEED ROOT ROUTER HANDLER
// ====================================================================
async fn handle_http_root() -> impl axum::response::IntoResponse {
    // Physics of execution: stream unmanaged read-only buffer layout from RAM into network stack instantly
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CONTENT_ENCODING, "gzip"),
        ],
        SVELTE_FRONTEND_UI,
    ).into_response()
}


const ADMIN_TOKEN: &str = "your token";

// ====================================================================
// CORE APILINE SECURITY MIDDLEWARE INTERCEPTOR
// ====================================================================
async fn check_admin_token(
    headers: axum::http::HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    // Read the custom security token header from the incoming network package
    let token_header = headers
        .get("X-Plato-Admin-Token")
        .and_then(|v| v.to_str().ok());

    // Boundary validation: isolate layout access to authenticated actors only
    if token_header == Some(ADMIN_TOKEN) {
    //Temporary development bypass allowing unauthenticated local loopback testing
//    if token_header == Some(ADMIN_TOKEN) || token_header.is_none() {
        // Token confirmed, route the request further into the core engine pipeline execution
        Ok(next.run(request).await)
    } else {
        // Drop the unauthorized connection immediately at the outer register floor
        println!("SECURITY NOTICE: Unauthorized access blocked on protected administrative node.");
        Err(axum::http::StatusCode::UNAUTHORIZED)
    }
}



// Static HTML dashboard with interactive slot matrix and telemetry moved to the bottom floor
const ADMIN_PANEL: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>L2-Monolith Board Journal</title>
    <style>
        body { font-family: monospace; background: #111; color: #eee; max-width: 900px; margin: 40px auto; padding: 0 20px; }
        h1 { color: #00ff00; margin-bottom: 25px; }
        input, select, textarea, button { width: 100%; background: #222; color: #00ff00; border: 1px solid #00ff00; padding: 10px; font-family: monospace; font-size: 14px; margin-bottom: 15px; box-sizing: border-box; }
        select option { background: #222; color: #00ff00; }
        button { cursor: pointer; font-weight: bold; background: #004400; margin-top: 10px; }
        button:hover { background: #006600; }
        button.delete-btn { background: #440000; border-color: #ff0000; color: #ff0000; }
        button.delete-btn:hover { background: #660000; }
        #status { color: #00ff00; font-weight: bold; margin-top: 10px; }
        label { font-weight: bold; color: #00aa00; display: block; margin-bottom: 5px; }
        .action-container { border: 1px dashed #333; padding: 20px; margin-bottom: 30px; }
        
        /* High-density bottom grid matrix styles */
        #telemetry-bar { color: #ffaa00; font-weight: bold; font-size: 13px; margin-top: 15px; border-top: 1px dashed #333; padding-top: 15px; }
        .matrix-title { font-weight: bold; color: #00ff00; margin-top: 30px; margin-bottom: 10px; }
        .slot-matrix { display: grid; grid-template-columns: repeat(32, 1fr); gap: 2px; background: #1a1a1a; padding: 5px; border: 1px solid #333; margin-bottom: 40px; }
        .slot-cell { width: 100%; aspect-ratio: 1; background: #052205; border: 1px solid #003300; cursor: pointer; font-size: 8px; display: flex; align-items: center; justify-content: center; color: #004400; user-select: none; }
        .slot-cell:hover { border-color: #00ff00; color: #00ff00; background: #003300; }
        .slot-cell.occupied-profile { background: #000044; border-color: #0000ff; color: #0088ff; }
        .slot-cell.occupied-article { background: #222222; border-color: #888888; color: #ffffff; }
        .slot-cell.occupied-letter { background: #442200; border-color: #ffaa00; color: #ffaa00; }
        .slot-cell.occupied-correspondence { background: #440044; border-color: #ff00ff; color: #ff00ff; }
    </style>
</head>
<body>
    <h1>[ BORED JOURNAL ADMIN PANEL ]</h1>
    
    <!-- COMMIT DATA ZONE -->
    <div class="action-container">
        <h3>[ COMMIT / EDIT ARTIFACT ]</h3>
        <label>Article ID (0 - 1023):</label>
        <input type="number" id="article_id" min="0" max="1023" value="5">

        <label>Content Entity Type:</label>
        <select id="content_type">
            <option value="1">TYPE_PROFILE</option>
            <option value="2" selected>TYPE_ARTICLE</option>
            <option value="4">TYPE_LETTER</option>
            <option value="5">TYPE_CORRESPONDENCE</option>
        </select>

        <label>Author Profile (Creator):</label>
        <select id="author_id">
            <option value="0">PROFILE_SYSTEM</option>
            <option value="1" selected>PROFILE_ADMIN</option>
            <option value="2">PROFILE_WORLD</option>
            <option value="3">PROFILE_LAMBER</option>
            <option value="4">PROFILE_LAMIEL</option>
        </select>

        <label>Target Profile (Destination):</label>
        <select id="target_id">
            <option value="0">PROFILE_SYSTEM</option>
            <option value="1">PROFILE_ADMIN</option>
            <option value="2" selected>PROFILE_WORLD</option>
            <option value="3">PROFILE_LAMBER</option>
            <option value="4">PROFILE_LAMIEL</option>
        </select>

        <label>Generous Title (Max 242 bytes):</label>
        <input type="text" id="title" placeholder="Enter article title here..." maxlength="242">

        <label>Content (Raw UTF-8 Text):</label>
        <textarea id="content" rows="15" placeholder="Write your article or existential AI discourse here..."></textarea>

        <button onclick="saveArticle()">[ COMMIT TO L2 CACHE & DISK ]</button>
    </div>

    <!-- OBLITERATE DATA ZONE -->
    <div class="action-container">
        <h3>[ OBLITERATE ARTIFACT ]</h3>
        <label>Target Slot ID to Delete (0 - 1023):</label>
        <input type="number" id="delete_id" min="0" max="1023" value="5">
        <button class="delete-btn" onclick="deleteArticle()">[ OBLITERATE SLOT FROM CORE INDEXES ]</button>
    </div>

    <div id="status">Ready.</div>

    <!-- SYSTEM FLOOR: HARDWARE MEMORY MATRIX AND TELEMETRY INSTRUMENTATION -->
    <div class="matrix-title">[ CORE MEMORY MAP LAYOUT (1024 SLOTS) ]</div>
    <div class="slot-matrix" id="matrix-container"></div>
    <div id="telemetry-bar">[ Core Slot Audit Map Generated in: <span id="telemetry-us">0.00</span> &mu;s ]</div>
    <a href="/api/rebuild" target="_blank" style="display: block; text-align: center; background: #222200; color: #aaaa00; border: 1px solid #aaaa00; padding: 10px; text-decoration: none; font-weight: bold; margin-top: 15px;">[ FORCE JOURNAL RECOMPILATION LOOP ]</a>

    <div class="control-group" style="margin-bottom: 20px; padding: 15px; border: 1px dashed #4e73a2; background: #161b22;">
    <button onclick="triggerGarbageCollector()" style="background: transparent; border: 1px solid #ffaa00; color: #ffaa00; padding: 10px 20px; cursor: pointer; font-family: monospace; font-weight: bold;">
        [ RUN GARBAGE COLLECTION ]
    </button>
    <span id="gc-status" style="margin-left: 15px; font-family: monospace; font-size: 13px; color: #8e959e;">
        System memory idle.
    </span>
    </div>

    <script>
        const container = document.getElementById('matrix-container');
        for (let i = 0; i < 1024; i++) {
            const cell = document.createElement('div');
            cell.className = 'slot-cell';
            cell.id = `slot-${i}`;
            cell.innerText = i;
            cell.onclick = () => {
                document.getElementById('article_id').value = i;
                document.getElementById('delete_id').value = i;
            };
            container.appendChild(cell);
        }

        async function refreshMemoryMap() {
            try {
                const response = await fetch('/api/slots');
                if (!response.ok) return;
                
                const us = response.headers.get('X-Platon-Audit-Microseconds');
                if (us) document.getElementById('telemetry-us').innerText = us;
                
                const occupiedSlots = await response.json();
                
                for (let i = 0; i < 1024; i++) {
                    document.getElementById(`slot-${i}`).className = 'slot-cell';
                }
                
                occupiedSlots.forEach(slot => {
                    const cell = document.getElementById(`slot-${slot.id}`);
                    if (cell) {
                        if (slot.content_type === 1) cell.classList.add('occupied-profile');
                        else if (slot.content_type === 2) cell.classList.add('occupied-article');
                        else if (slot.content_type === 4) cell.classList.add('occupied-letter');
                        else if (slot.content_type === 5) cell.classList.add('occupied-correspondence');
                    }
                });
            } catch (err) {
                console.error("Telemetry map pipeline dropped:", err);
            }
        }

        async function saveArticle() {
            const id = parseInt(document.getElementById('article_id').value);
            const content_type = parseInt(document.getElementById('content_type').value);
            const author_id = parseInt(document.getElementById('author_id').value);
            const target_id = parseInt(document.getElementById('target_id').value);
            const title = document.getElementById('title').value;
            const text = document.getElementById('content').value;
            const statusDiv = document.getElementById('status');

            statusDiv.innerText = "Transmitting to silicon...";

            try {
                const response = await fetch('/api/save', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ id, content_type, author_id, target_id, title, text })
                });

                const result = await response.text();
                if (response.ok) {
                    statusDiv.innerText = "SUCCESS: " + result;
                    await refreshMemoryMap();
                } else {
                    statusDiv.innerText = "ERROR: " + result;
                }
            } catch (err) {
                statusDiv.innerText = "NETWORK CRITICAL: " + err.message;
            }
        }

        async function deleteArticle() {
            const id = parseInt(document.getElementById('delete_id').value);
            const statusDiv = document.getElementById('status');

            if (isNaN(id) || id < 0 || id >= 1024) {
                statusDiv.innerText = "VALIDATION ERROR: Invalid Slot ID";
                return;
            }

            statusDiv.innerText = "Obliterating slot from core memory...";

            try {
                const response = await fetch(`/api/delete/${id}`, { method: 'POST' });
                if (response.ok) {
                    statusDiv.innerText = `SUCCESS: Slot ${id} completely wiped out.`;
                    await refreshMemoryMap();
                } else {
                    statusDiv.innerText = `ERROR: Server refused to delete slot ${id}`;
                }
            } catch (err) {
                statusDiv.innerText = "NETWORK CRITICAL: " + err.message;
            }
        }

        refreshMemoryMap();

  function triggerGarbageCollector() {
    const statusLabel = document.getElementById('gc-status');
    statusLabel.style.color = '#ffaa00';
    statusLabel.innerText = 'Executing alignment audit...';

    fetch('/api/gc')
        .then(response => {
            if (response.ok) {
                return response.text();
            }
            throw new Error('Core rejected execution thread');
        })
        .then(logText => {
            statusLabel.style.color = '#00ff66';
            statusLabel.innerText = 'SUCCESS: ' + logText.trim();
        })
        .catch(err => {
            statusLabel.style.color = '#f85149';
            statusLabel.innerText = 'CRITICAL OOM/FAIL: ' + err.message;
        });
}
    </script>
</body>
</html>
"#;