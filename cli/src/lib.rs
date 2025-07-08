// Fix for the rpassword dependency issue
use std::io::{self, Write};

pub fn read_password() -> io::Result<String> {
    print!("Password: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_string())
}

// Re-export the function for use in other modules
pub mod rpassword {
    pub fn read_password() -> std::io::Result<String> {
        crate::read_password()
    }
}