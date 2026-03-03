use anchor_lang::prelude::*;

declare_id!("BUxg7dR7avAMivxrXizTgj64LQEiYrk8Qjt3xSN4JRDc");

pub const READ: u64 = 1 << 0;
pub const WRITE: u64 = 1 << 1;
pub const DELETE: u64 = 1 << 2;
pub const ADMIN: u64 = 1 << 3;
pub const WEBHOOK: u64 = 1 << 4;

#[program]
pub mod api_key_management {
    use super::*;

    pub fn initialize_registry(ctx: Context<InitializeRegistry>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        registry.authority = ctx.accounts.authority.key();
        registry.total_keys = 0;
        registry.active_keys = 0;
        registry.bump = ctx.bumps.registry;
        Ok(())
    }

    pub fn create_api_key(
        ctx: Context<CreateApiKey>,
        key_id: u64,
        name: String,
        key_hash: [u8; 32],
        permissions: u64,
        expires_at: Option<i64>,
        metadata: String,
    ) -> Result<()> {
        require!(name.len() <= 64, ApiKeyError::NameTooLong);
        require!(metadata.len() <= 128, ApiKeyError::MetadataTooLong);
        require!(permissions != 0, ApiKeyError::InvalidPermissions);

        let now = Clock::get()?.unix_timestamp;
        if let Some(exp) = expires_at {
            require!(exp > now, ApiKeyError::InvalidExpiry);
        }

        let key = &mut ctx.accounts.api_key;
        key.owner = ctx.accounts.owner.key();
        key.key_id = key_id;
        key.key_hash = key_hash;
        key.name = name;
        key.permissions = permissions;
        key.created_at = now;
        key.expires_at = expires_at;
        key.last_used_at = now;
        key.usage_count = 0;
        key.is_active = true;
        key.metadata = metadata;
        key.bump = ctx.bumps.api_key;

        let registry = &mut ctx.accounts.registry;
        registry.total_keys = registry.total_keys.checked_add(1).ok_or(ApiKeyError::Overflow)?;
        registry.active_keys = registry.active_keys.checked_add(1).ok_or(ApiKeyError::Overflow)?;

        emit!(ApiKeyVerifiedEvent {
            key: key.key(),
            key_id,
            valid: true,
            timestamp: now,
        });

        Ok(())
    }

    pub fn revoke_api_key(ctx: Context<OwnerMutateApiKey>, key_id: u64) -> Result<()> {
        let _ = key_id;
        let key = &mut ctx.accounts.api_key;
        require!(key.owner == ctx.accounts.owner.key(), ApiKeyError::Unauthorized);
        require!(key.is_active, ApiKeyError::KeyNotActive);
        key.is_active = false;

        let registry = &mut ctx.accounts.registry;
        registry.active_keys = registry.active_keys.checked_sub(1).ok_or(ApiKeyError::Overflow)?;
        Ok(())
    }

    pub fn rotate_api_key(
        ctx: Context<OwnerMutateApiKey>,
        key_id: u64,
        new_key_hash: [u8; 32],
    ) -> Result<()> {
        let _ = key_id;
        let key = &mut ctx.accounts.api_key;
        require!(key.owner == ctx.accounts.owner.key(), ApiKeyError::Unauthorized);
        require!(key.is_active, ApiKeyError::KeyNotActive);
        key.key_hash = new_key_hash;
        Ok(())
    }

    pub fn update_permissions(
        ctx: Context<OwnerMutateApiKey>,
        key_id: u64,
        new_permissions: u64,
    ) -> Result<()> {
        let _ = key_id;
        let key = &mut ctx.accounts.api_key;
        require!(key.owner == ctx.accounts.owner.key(), ApiKeyError::Unauthorized);
        require!(key.is_active, ApiKeyError::KeyNotActive);
        require!(new_permissions != 0, ApiKeyError::InvalidPermissions);
        key.permissions = new_permissions;
        Ok(())
    }

    pub fn record_usage(ctx: Context<RecordUsage>, key_id: u64, required_flag: u64) -> Result<()> {
        let _ = key_id;
        let key = &mut ctx.accounts.api_key;
        require!(key.is_valid(), ApiKeyError::KeyNotActive);
        require!(has_permission(key.permissions, required_flag), ApiKeyError::InsufficientPermissions);

        let now = Clock::get()?.unix_timestamp;
        key.last_used_at = now;
        key.usage_count = key.usage_count.checked_add(1).ok_or(ApiKeyError::Overflow)?;
        Ok(())
    }

    pub fn verify_api_key(
        ctx: Context<VerifyApiKey>,
        key_id: u64,
        provided_hash: [u8; 32],
    ) -> Result<()> {
        let _ = key_id;
        let key = &ctx.accounts.api_key;

        if !key.is_active {
            emit!(ApiKeyVerifiedEvent {
                key: key.key(),
                key_id: key.key_id,
                valid: false,
                timestamp: Clock::get()?.unix_timestamp,
            });
            return err!(ApiKeyError::KeyNotActive);
        }
        if key.is_expired() {
            emit!(ApiKeyVerifiedEvent {
                key: key.key(),
                key_id: key.key_id,
                valid: false,
                timestamp: Clock::get()?.unix_timestamp,
            });
            return err!(ApiKeyError::KeyExpired);
        }
        if key.key_hash != provided_hash {
            emit!(ApiKeyVerifiedEvent {
                key: key.key(),
                key_id: key.key_id,
                valid: false,
                timestamp: Clock::get()?.unix_timestamp,
            });
            return err!(ApiKeyError::InvalidKeyHash);
        }

        emit!(ApiKeyVerifiedEvent {
            key: key.key(),
            key_id: key.key_id,
            valid: true,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn close_api_key(ctx: Context<CloseApiKey>, key_id: u64) -> Result<()> {
        let _ = key_id;
        let key = &ctx.accounts.api_key;
        require!(key.owner == ctx.accounts.owner.key(), ApiKeyError::Unauthorized);
        require!(!key.is_active, ApiKeyError::CloseActiveKey);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeRegistry<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + RegistryAccount::INIT_SPACE,
        seeds = [b"registry", authority.key().as_ref()],
        bump
    )]
    pub registry: Account<'info, RegistryAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(key_id: u64)]
pub struct CreateApiKey<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [b"registry", owner.key().as_ref()],
        bump = registry.bump,
        has_one = authority @ ApiKeyError::Unauthorized
    )]
    pub registry: Account<'info, RegistryAccount>,
    /// CHECK: constrained by has_one
    pub authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = owner,
        space = 8 + ApiKeyAccount::INIT_SPACE,
        seeds = [b"api_key", owner.key().as_ref(), &key_id.to_le_bytes()],
        bump
    )]
    pub api_key: Account<'info, ApiKeyAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(key_id: u64)]
pub struct OwnerMutateApiKey<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [b"registry", owner.key().as_ref()],
        bump = registry.bump
    )]
    pub registry: Account<'info, RegistryAccount>,
    #[account(
        mut,
        seeds = [b"api_key", owner.key().as_ref(), &key_id.to_le_bytes()],
        bump = api_key.bump
    )]
    pub api_key: Account<'info, ApiKeyAccount>,
}

#[derive(Accounts)]
#[instruction(key_id: u64)]
pub struct RecordUsage<'info> {
    /// CHECK: caller can be anyone
    pub caller: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"api_key", caller.key().as_ref(), &key_id.to_le_bytes()],
        bump = api_key.bump
    )]
    pub api_key: Account<'info, ApiKeyAccount>,
}

#[derive(Accounts)]
#[instruction(key_id: u64)]
pub struct VerifyApiKey<'info> {
    /// CHECK: caller can be anyone
    pub caller: UncheckedAccount<'info>,
    #[account(
        seeds = [b"api_key", caller.key().as_ref(), &key_id.to_le_bytes()],
        bump = api_key.bump
    )]
    pub api_key: Account<'info, ApiKeyAccount>,
}

#[derive(Accounts)]
#[instruction(key_id: u64)]
pub struct CloseApiKey<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        close = owner,
        seeds = [b"api_key", owner.key().as_ref(), &key_id.to_le_bytes()],
        bump = api_key.bump
    )]
    pub api_key: Account<'info, ApiKeyAccount>,
}

#[account]
#[derive(InitSpace)]
pub struct RegistryAccount {
    pub authority: Pubkey,
    pub total_keys: u64,
    pub active_keys: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ApiKeyAccount {
    pub owner: Pubkey,
    pub key_id: u64,
    pub key_hash: [u8; 32],
    #[max_len(64)]
    pub name: String,
    pub permissions: u64,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used_at: i64,
    pub usage_count: u64,
    pub is_active: bool,
    #[max_len(128)]
    pub metadata: String,
    pub bump: u8,
}

impl ApiKeyAccount {
    pub fn is_expired_at(&self, now: i64) -> bool {
        self.expires_at.map(|exp| now > exp).unwrap_or(false)
    }

    pub fn is_expired(&self) -> bool {
        Clock::get()
            .map(|c| self.is_expired_at(c.unix_timestamp))
            .unwrap_or(false)
    }

    pub fn is_valid_at(&self, now: i64) -> bool {
        self.is_active && !self.is_expired_at(now)
    }

    pub fn is_valid(&self) -> bool {
        self.is_active && !self.is_expired()
    }
}

#[account]
#[derive(InitSpace)]
pub struct AuditLogAccount {
    pub api_key: Pubkey,
    pub action: AuditAction,
    pub actor: Pubkey,
    pub timestamp: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub enum AuditAction {
    Create,
    Revoke,
    Rotate,
    UpdatePermissions,
    RecordUsage,
    Verify,
    Close,
}

#[event]
pub struct ApiKeyVerifiedEvent {
    pub key: Pubkey,
    pub key_id: u64,
    pub valid: bool,
    pub timestamp: i64,
}

pub fn has_permission(perms: u64, flag: u64) -> bool {
    perms & flag == flag
}

#[error_code]
pub enum ApiKeyError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("API key is not active")]
    KeyNotActive,
    #[msg("API key is expired")]
    KeyExpired,
    #[msg("Invalid API key hash")]
    InvalidKeyHash,
    #[msg("Insufficient permissions")]
    InsufficientPermissions,
    #[msg("Registry/account overflow")]
    Overflow,
    #[msg("Name too long")]
    NameTooLong,
    #[msg("Metadata too long")]
    MetadataTooLong,
    #[msg("Invalid expiry")]
    InvalidExpiry,
    #[msg("Invalid permissions")]
    InvalidPermissions,
    #[msg("Cannot close active key")]
    CloseActiveKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_api_key(expires_at: Option<i64>, is_active: bool) -> ApiKeyAccount {
        ApiKeyAccount {
            owner: Pubkey::default(),
            key_id: 1,
            key_hash: [7u8; 32],
            name: "demo".to_string(),
            permissions: READ | WRITE,
            created_at: 100,
            expires_at,
            last_used_at: 100,
            usage_count: 0,
            is_active,
            metadata: "test".to_string(),
            bump: 255,
        }
    }

    #[test]
    fn permission_single_flag_true() {
        assert!(has_permission(READ | WRITE, READ));
    }

    #[test]
    fn permission_single_flag_false() {
        assert!(!has_permission(READ | WRITE, DELETE));
    }

    #[test]
    fn permission_combined_flags_true() {
        assert!(has_permission(READ | WRITE | ADMIN, READ | ADMIN));
    }

    #[test]
    fn permission_combined_flags_false() {
        assert!(!has_permission(READ | WRITE, READ | DELETE));
    }

    #[test]
    fn key_without_expiry_is_not_expired() {
        let key = sample_api_key(None, true);
        assert!(!key.is_expired_at(1_000_000));
    }

    #[test]
    fn key_before_expiry_is_not_expired() {
        let key = sample_api_key(Some(200), true);
        assert!(!key.is_expired_at(199));
    }

    #[test]
    fn key_after_expiry_is_expired() {
        let key = sample_api_key(Some(200), true);
        assert!(key.is_expired_at(201));
    }

    #[test]
    fn inactive_key_is_not_valid_even_without_expiry() {
        let key = sample_api_key(None, false);
        assert!(!key.is_valid_at(100));
    }

    #[test]
    fn active_non_expired_key_is_valid() {
        let key = sample_api_key(Some(300), true);
        assert!(key.is_valid_at(250));
    }

    #[test]
    fn active_expired_key_is_not_valid() {
        let key = sample_api_key(Some(300), true);
        assert!(!key.is_valid_at(301));
    }
}
