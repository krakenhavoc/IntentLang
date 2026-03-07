# Authentication

A user authentication system with password-based login, session management, and brute-force protection. Demonstrates existential quantifiers, domain functions, and rate limiting.

**File:** [`examples/auth.intent`](https://github.com/krakenhavoc/IntentLang/blob/main/examples/auth.intent)

```intent
module Authentication

--- User authentication system with password-based login,
--- session management, and brute-force protection.

entity User {
  id: UUID
  email: Email
  password_hash: String
  status: Active | Suspended | Deactivated
  failed_attempts: Int
  locked_until: DateTime?
  last_login: DateTime?
  created_at: DateTime
}

entity Session {
  id: UUID
  user: User
  token: String
  expires_at: DateTime
  created_at: DateTime
  revoked: Bool
}

action Login {
  --- Authenticate a user with email and password.
  email: Email
  password: String

  requires {
    exists u: User => u.email == email
    lookup(User, email).status == Active
    lookup(User, email).locked_until == null ||
      lookup(User, email).locked_until < now()
  }

  ensures {
    when password_verified(password, lookup(User, email).password_hash) =>
      exists s: Session =>
        s.user.email == email &&
        s.revoked == false &&
        s.expires_at > now()
      &&
      lookup(User, email).failed_attempts == 0
      &&
      lookup(User, email).last_login == now()

    when !password_verified(password, lookup(User, email).password_hash) =>
      lookup(User, email).failed_attempts ==
        old(lookup(User, email).failed_attempts) + 1
  }

  properties {
    audit_logged: true
    rate_limited: { max: 10, window_seconds: 60, key: email }
    sensitive_fields: [password]
  }
}

action Logout {
  --- End a user session.
  session: Session

  requires {
    session.revoked == false
    session.expires_at > now()
  }

  ensures {
    session.revoked == true
  }

  properties {
    idempotent: true
    audit_logged: true
  }
}

invariant MaxFailedAttempts {
  --- Lock accounts after 5 consecutive failed login attempts.
  forall u: User =>
    u.failed_attempts >= 5 => u.locked_until != null
}

invariant SessionExpiry {
  --- All active sessions must have a future expiration.
  forall s: Session =>
    s.revoked == false => s.expires_at > now()
}

edge_cases {
  when lookup(User, email).status == Suspended =>
    reject("Account suspended. Contact support.")
  when lookup(User, email).status == Deactivated =>
    reject("Account deactivated.")
  when lookup(User, email).failed_attempts >= 5 =>
    reject("Account locked due to too many failed attempts.")
}
```

## Key concepts demonstrated

- **Domain functions** like `lookup()`, `now()`, `password_verified()`
- **Conditional postconditions** using `when ... =>` in ensures blocks
- **Existential quantifiers** in both requires and ensures
- **Implication in invariants** (`failed_attempts >= 5 => locked_until != null`)
- **Rate limiting and sensitive fields** as action properties
