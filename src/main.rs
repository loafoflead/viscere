use std::{io, collections::HashMap, path::{Path, PathBuf}, fs::{Metadata, self, DirEntry}};

type Tag<'a> = &'a str;

struct Project<'a> {
    /// The where the files are stored
    root:   PathBuf,
    /// Map of tags correlated to their quantity ()
    tags:   HashMap<Tag<'a>, u32>,
    /// Stands for 'file system', where the project struct 
    /// stores files and folders with their associates tags.
    fs:     Option<FsItem>, 
}

#[derive(Eq, PartialEq, Hash, Debug)]
enum FsItem {
    Folder {
        name: String, 
        tags: Vec<String>, 
        children: Vec<Box<FsItem>>
    },
    File {
        name: String, 
        tags: Vec<String>
    },
}

impl FsItem {
    fn from_dir(path: &Path) -> Option<Self> {
        let md = path.metadata().ok()?;
        if !md.is_dir() {
            return None;
        }

        let entries = fs::read_dir(path).ok()?;

        let children = entries
            .into_iter()
            .flat_map(|e| {
                Some(Box::new(Self::from_entry(e.ok()?)?))
            })
            .collect();

        Some(Self::Folder {
            name: path.file_name()?.to_str()?.to_owned(), 
            tags: vec!(),
            children
        })
    }

    fn from_entry(entry: DirEntry) -> Option<Self> {
        let md = entry.metadata().ok()?;

        let name = entry.path().to_str().map(|s| s.to_string())?;

        Some(
            if md.is_file() {
                Self::File {name, tags: vec!()}
            }
            else if md.is_dir() {
                Self::from_dir(&entry.path())? 
            }
            else { return None; }
        )
    }
}

fn main() {
    println!("Welcome to viscere. Type 'help' for help.");

    let mut curr_prog: Option<Project> = None;
    
    'main: loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let argv = input.trim().split_whitespace().collect::<Vec<_>>();

        if argv.len() < 1 {
            println!();
            continue 'main;
        }

        if argv.len() == 1 {
            match argv[0] {
                "q" | "quit" => break 'main,
                "?" | "help" => println!("Commands: {:?}", CMDS),
                "ls" | "list" => todo!("Implement viewing of current project."),
                cmd => unknown_command(cmd)
            }
        }

        if argv.len() == 2 {
            match argv[0] {
                "a" | "add" => todo!("Implement adding files or folders to project."),
                "n" | "new" => {
                    curr_prog = Some(Project { root: PathBuf::from("."), tags: HashMap::new(), fs: None });
                    
                    curr_prog.unwrap().fs = Some(FsItem::from_dir(&PathBuf::from(".")).unwrap());

                    println!("Successfully initialised project in current directory, all files indexed.")
                }
                "?" | "help" => print_help(argv[1]), 
                cmd => unknown_command(cmd)
            }
        }

        if argv.len() == 3 {
            match argv[0] {
                "n" | "new" => todo!("Implement creating new project."),
                cmd => unknown_command(cmd)
            }
        }
    }
}

const HELP: &[&str] = &[
    "Unknown help option, be sure to type the full name of the command, and not an alias.",
    "quit:  Quits the program.",
    "list:  Lists the stats of the current project.",
    "add:   Adds a file or folder to the current project. Usage: add <file/folder name>",
    "help:  Displays this message. Usage: help (<command name>)",
    "new:   Creates a new project with an optionally given root path (default is current path) and mandatory name, forgetting the old one. Usage: new <name> (<root>)",
];

const CMDS: &[&str] = &[
    "quit", "list", "add", "help", "new",
];

fn index_of<T: PartialEq>(haystack: &[T], needle: T) -> Option<usize> {
    for (i, item) in haystack.into_iter().enumerate() {
        if *item == needle {
            return Some(i);
        }
    }
    None
}

fn print_help(cmd: &str) {
    let idx = index_of(CMDS, cmd).unwrap_or(0);
    println!("{}", HELP[idx + 1]);
}

fn unknown_command(cmd: &str) {
    println!("Unknown command: `{cmd}`");
}
