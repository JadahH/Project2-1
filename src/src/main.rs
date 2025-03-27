use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

const MEMORY_SIZE: usize = 65535; // Define total memory size

#[derive(Debug)]
#[allow(dead_code)]
struct MemoryBlock {
    start: usize,
    size: usize,
    allocated: bool,
    id: Option<usize>,
}

struct MemoryManager {
    memory: [u8; MEMORY_SIZE],
    free_blocks: BTreeMap<usize, Vec<MemoryBlock>>, // Best-fit allocation
    allocated_blocks: BTreeMap<usize, MemoryBlock>, // Tracks allocated blocks
    next_id: usize, // Unique ID for allocations
}

impl MemoryManager {
    fn new() -> Self {
        let mut free_blocks = BTreeMap::new();
        free_blocks.insert(MEMORY_SIZE, vec![MemoryBlock {
            start: 0,
            size: MEMORY_SIZE,
            allocated: false,
            id: None,
        }]);

        Self {
            memory: [0; MEMORY_SIZE],
            free_blocks,
            allocated_blocks: BTreeMap::new(),
            next_id: 0,
        }
    }

    fn insert(&mut self, size: usize, data: &[u8]) -> Option<usize> {
        let best_fit = self.free_blocks
            .iter_mut()
            .filter(|(block_size, _)| **block_size >= size)
            .min_by_key(|(block_size, _)| **block_size);


        if let Some((&block_size, blocks)) = best_fit {
            if let Some(block) = blocks.pop() {
                if blocks.is_empty() {
                    self.free_blocks.remove(&block_size);
                }

                let new_id = self.next_id;
                self.next_id += 1;

                self.memory[block.start..block.start + size].copy_from_slice(&data);

                let allocated_block = MemoryBlock {
                    start: block.start,
                    size,
                    allocated: true,
                    id: Some(new_id),
                };
                self.allocated_blocks.insert(new_id, allocated_block);

                // Handle leftover memory in the block
                if block.size > size {
                    let remaining_block = MemoryBlock {
                        start: block.start + size,
                        size: block.size - size,
                        allocated: false,
                        id: None,
                    };
                    self.free_blocks.entry(block.size - size).or_insert_with(Vec::new).push(remaining_block);
                }

                return Some(new_id);
            }
        }
        None
    }

    fn delete(&mut self, id: usize) {
        if let Some(block) = self.allocated_blocks.remove(&id) {
            let new_block = MemoryBlock {
                start: block.start,
                size: block.size,
                allocated: false,
                id: None,
            };
            self.free_blocks.entry(block.size).or_insert_with(Vec::new).push(new_block);
            println!("Deleted ID: {}", id);
        } else {
            println!("Error: ID {} not found", id);
        }
    }

    fn find(&self, id: usize) -> Option<&[u8]> {
        self.allocated_blocks.get(&id).map(|block| {
            &self.memory[block.start..block.start + block.size]
        })
    }
    fn read(&self, id: usize) {
        match self.allocated_blocks.get(&id) {
            Some(data) => println!("Data at ID {}: {:?}", id, data),
            None => println!("Error: ID {} not found", id),
        }
    }

    fn update(&mut self, id: usize, new_data: &[u8]) {
        if let Some(block) = self.allocated_blocks.get_mut(&id) {
            if new_data.len() <= block.size {
                self.memory[block.start..block.start + new_data.len()].copy_from_slice(new_data);
                println!("Updated ID: {} with new data {:?}", id, new_data);
            } else {
                println!("Error: New data exceeds allocated block size");
            }
        } else {
            println!("Error: ID {} not found", id);
        }
    }
 
    fn dump(&self) {
        println!("Memory Dump:");
        for (size, blocks) in &self.free_blocks {
            for block in blocks {
                println!("FREE: Start: {:#06x}, Size: {}", block.start, size);
            }
        }
        for (id, block) in &self.allocated_blocks {
            println!("ALLOCATED: ID: {}, Start: {:#06x}, Size: {}", id, block.start, block.size);
        }
    }
}


fn process_file(file_path: &str, memory_manager: &mut MemoryManager) -> io::Result<()> {
    if let Ok(lines) = read_lines(file_path) {
        for line in lines.flatten() {
            println!("Processing line: {}", line); // Debug print
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }


            match tokens[0] {
                "INSERT" => {
                    if tokens.len() < 3 {
                        println!("Error: Invalid INSERT command");
                        continue;
                    }
                    if let (Ok(size), data) = (tokens[1].parse::<usize>(), tokens[2].as_bytes()) {
                        if let Some(id) = memory_manager.insert(size, data) {
                            println!("Allocated ID: {}", id);
                        } else {
                            println!("Memory allocation failed");
                        }
                    }
                }
                "DELETE" => {
                    if tokens.len() < 2 {
                        println!("Error: Invalid DELETE command");
                        continue;
                    }
                    if let Ok(id) = tokens[1].parse::<usize>() {
                        memory_manager.delete(id);
                    }
                }
                "FIND" => {
                    if tokens.len() < 2 {
                        println!("Error: Invalid FIND command");
                        continue;
                    }
                    if let Ok(id) = tokens[1].parse::<usize>() {
                        if let Some(data) = memory_manager.find(id) {
                            println!("Data at {}: {:?}", id, data);
                        } else {
                            println!("Nothing at {}", id);
                        }
                    }
                }

                "READ" if tokens.len() == 2 => {
                    if let Ok(id) = tokens[1].parse::<usize>() {
                        memory_manager.read(id);
                    } else {
                        println!("Invalid READ command format");
                    }
                }
                
                "UPDATE" => {
                    if tokens.len() < 3 {
                        println!("Error: Invalid UPDATE command");
                        continue;
                    }
                    if let (Ok(id), new_data) = (tokens[1].parse::<usize>(), tokens[2].as_bytes()) {
                        memory_manager.update(id, new_data);
                    }
                }





                "DUMP" => {
                    memory_manager.dump();
                }
                _ => {
                    println!("Error: Unknown command `{}`", tokens[0]);
                }
            }
        }
    }
    Ok(())
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
fn main() {
    let mut memory_manager = MemoryManager::new();
    let file_path = "commands.cmmd"; 
    if let Err(err) = process_file(file_path, &mut memory_manager) {
        eprintln!("Error processing file: {}", err);
    }
}
