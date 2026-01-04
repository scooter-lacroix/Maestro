"""
Data Sanitization

Sanitize sensitive data from memory content before storage.
"""

import re
from typing import List, Tuple

class MemorySanitizer:
    """Sanitize sensitive data from memory content"""

    # Issue 11: Expanded patterns to cover JWT, certificates, AWS keys, GitHub tokens
    SENSITIVE_PATTERNS: List[Tuple[str, str]] = [
        # API keys (various formats)
        (r'API[_-]?KEY\s*[:=]\s*[\'"]?[a-zA-Z0-9_\-]{20,}', '***'),
        (r'"api_key"\s*:\s*"[^"]{20,}"', '"api_key": "***"'),
        (r'"apiKey"\s*:\s*"[^"]{20,}"', '"apiKey": "***"'),

        # Tokens
        (r'TOKEN\s*[:=]\s*[\'"]?[a-zA-Z0-9_\-]{20,}', '***'),
        (r'"token"\s*:\s*"[^"]{20,}"', '"token": "***"'),
        (r'"accessToken"\s*:\s*"[^"]{20,}"', '"accessToken": "***"'),
        (r'"refreshToken"\s*:\s*"[^"]{20,}"', '"refreshToken": "***"'),

        # Issue 11: JWT tokens (header.payload.signature format)
        (r'eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+', '***JWT***'),
        (r'"jwt"\s*:\s*"[^"]{20,}"', '"jwt": "***"'),
        (r'"idToken"\s*:\s*"[^"]{20,}"', '"idToken": "***"'),

        # Issue 11: AWS credentials
        (r'AKIA[0-9A-Z]{16}', '***AWS***'),  # AWS Access Key ID
        (r'"aws_access_key_id"\s*:\s*"[^"]{20,}"', '"aws_access_key_id": "***"'),
        (r'"aws_secret_access_key"\s*:\s*"[^"]{20,}"', '"aws_secret_access_key": "***"'),
        (r'aws_access_key_id\s*=\s*[A-Z0-9]{20}', 'aws_access_key_id=***'),
        (r'aws_secret_access_key\s*=\s*[A-Za-z0-9/+=]{40}', 'aws_secret_access_key=***'),

        # Issue 11: GitHub tokens
        (r'ghp_[a-zA-Z0-9]{36}', '***GITHUB***'),  # GitHub Personal Access Token
        (r'gho_[a-zA-Z0-9]{36}', '***GITHUB***'),  # GitHub OAuth Access Token
        (r'ghu_[a-zA-Z0-9]{36}', '***GITHUB***'),  # GitHub User-to-Server Token
        (r'ghs_[a-zA-Z0-9]{36}', '***GITHUB***'),  # GitHub Server-to-Server Token
        (r'ghr_[a-zA-Z0-9]{36}', '***GITHUB***'),  # GitHub Refresh Token
        (r'github_[a-zA-Z0-9_]{36,}', '***GITHUB***'),

        # Issue 11: SSL/TLS Certificates and private keys
        (r'-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----', '***PRIVATE KEY***'),
        (r'-----BEGIN\s+CERTIFICATE-----', '***CERTIFICATE***'),
        (r'-----BEGIN\s+OPENSSH\s+PRIVATE\s+KEY-----', '***PRIVATE KEY***'),
        (r'ssh-rsa\s+[A-Za-z0-9/+=]+', 'ssh-rsa ***'),
        (r'ssh-ed25519\s+[A-Za-z0-9/+=]+', 'ssh-ed25519 ***'),

        # Passwords
        (r'PASSWORD\s*[:=]\s*[\'"]?[^\s\'"]{8,}', '***'),
        (r'"password"\s*:\s*"[^"]{8,}"', '"password": "***"'),
        (r'"passwd"\s*:\s*"[^"]{8,}"', '"passwd": "***"'),
        (r'"userPassword"\s*:\s*"[^"]{8,}"', '"userPassword": "***"'),

        # Secrets
        (r'SECRET\s*[:=]\s*[\'"]?[a-zA-Z0-9_\-]{20,}', '***'),
        (r'"secret"\s*:\s*"[^"]{20,}"', '"secret": "***"'),
        (r'"clientSecret"\s*:\s*"[^"]{20,}"', '"clientSecret": "***"'),
        (r'"client_secret"\s*:\s*"[^"]{20,}"', '"client_secret": "***"'),

        # Private keys
        (r'"private_key"\s*:\s*"[^"]{20,}"', '"private_key": "***"'),
        (r'"privateKey"\s*:\s*"[^"]{20,}"', '"privateKey": "***"'),

        # Issue 11: Additional sensitive patterns
        (r'"auth_token"\s*:\s*"[^"]{20,}"', '"auth_token": "***"'),
        (r'"authorization"\s*:\s*"[Bb]earer\s+[^\"]+\"', '"authorization": "Bearer ***"'),
        (r'"x_api_key"\s*:\s*"[^"]{20,}"', '"x_api_key": "***"'),
        (r'"x-api-key"\s*:\s*"[^"]{20,}"', '"x-api-key": "***"'),

        # Issue 11: Database connection strings
        (r'mongodb://[^\s"\'<>]+:[^\s"\'<>]+@', 'mongodb://***:***@'),
        (r'postgres://[^\s"\'<>]+:[^\s"\'<>]+@', 'postgres://***:***@'),
        (r'mysql://[^\s"\'<>]+:[^\s"\'<>]+@', 'mysql://***:***@'),
        (r'redis://:[^\s"\'<>]+@', 'redis://:***@'),
    ]

    @classmethod
    def sanitize(cls, content: str) -> str:
        """
        Remove sensitive data from content

        Args:
            content: Raw content

        Returns:
            Sanitized content
        """
        sanitized = content

        for pattern, replacement in cls.SENSITIVE_PATTERNS:
            sanitized = re.sub(pattern, replacement, sanitized, flags=re.IGNORECASE)

        return sanitized
