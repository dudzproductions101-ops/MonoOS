//! account_manager.rs – OneOS Framework Account Manager
//!
//! Manages user accounts added to the device (Google, OneOS, custom
//! authenticator-plugin accounts).  Provides token refresh, account
//! sync scheduling, and the credential vault interface.

use std::collections::HashMap;

/// Account type identifier (reverse-DNS style).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountType(pub String);

/// A single account record.
#[derive(Debug, Clone)]
pub struct Account {
    pub name:         String,
    pub account_type: AccountType,
    pub uid:          u32,   // device user this account belongs to
}

impl Account {
    pub fn new(name: impl Into<String>, account_type: impl Into<String>, uid: u32) -> Self {
        Account { name: name.into(), account_type: AccountType(account_type.into()), uid }
    }
}

/// An auth token for a specific scope.
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token:      String,
    pub scope:      String,
    pub expires_at: u64,   // Unix seconds; 0 = no expiry
}

impl AuthToken {
    pub fn is_expired(&self, now_secs: u64) -> bool {
        self.expires_at > 0 && now_secs >= self.expires_at
    }
}

/// The account manager — one instance per device user.
pub struct AccountManager {
    accounts: Vec<Account>,
    tokens:   HashMap<(String, String), AuthToken>, // (name, scope) → token
}

impl AccountManager {
    pub fn new() -> Self {
        AccountManager { accounts: Vec::new(), tokens: HashMap::new() }
    }

    pub fn add_account(&mut self, account: Account) -> Result<(), &'static str> {
        let dup = self.accounts.iter().any(|a|
            a.name == account.name && a.account_type == account.account_type);
        if dup { return Err("account already exists"); }
        self.accounts.push(account);
        Ok(())
    }

    pub fn remove_account(&mut self, name: &str, account_type: &str) -> bool {
        let before = self.accounts.len();
        self.accounts.retain(|a| !(a.name == name && a.account_type.0 == account_type));
        // Also remove tokens.
        self.tokens.retain(|(n, _), _| n != name);
        self.accounts.len() < before
    }

    pub fn accounts_for_uid(&self, uid: u32) -> Vec<&Account> {
        self.accounts.iter().filter(|a| a.uid == uid).collect()
    }

    pub fn accounts_by_type(&self, account_type: &str) -> Vec<&Account> {
        self.accounts.iter().filter(|a| a.account_type.0 == account_type).collect()
    }

    pub fn store_token(&mut self, name: &str, scope: &str, token: AuthToken) {
        self.tokens.insert((name.to_owned(), scope.to_owned()), token);
    }

    pub fn get_token(&self, name: &str, scope: &str, now_secs: u64) -> Option<&AuthToken> {
        let tok = self.tokens.get(&(name.to_owned(), scope.to_owned()))?;
        if tok.is_expired(now_secs) { return None; }
        Some(tok)
    }

    pub fn invalidate_token(&mut self, name: &str, scope: &str) {
        self.tokens.remove(&(name.to_owned(), scope.to_owned()));
    }

    pub fn account_count(&self) -> usize { self.accounts.len() }
}

impl Default for AccountManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove() {
        let mut mgr = AccountManager::new();
        mgr.add_account(Account::new("alice@example.com", "com.google", 0)).unwrap();
        assert_eq!(mgr.account_count(), 1);
        assert!(mgr.remove_account("alice@example.com", "com.google"));
        assert_eq!(mgr.account_count(), 0);
    }

    #[test]
    fn token_expiry() {
        let mut mgr = AccountManager::new();
        mgr.store_token("user", "email", AuthToken {
            token: "tok".into(), scope: "email".into(), expires_at: 1000
        });
        assert!(mgr.get_token("user", "email", 999).is_some());
        assert!(mgr.get_token("user", "email", 1000).is_none());
    }
}
