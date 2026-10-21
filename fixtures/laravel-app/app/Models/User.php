<?php

namespace App\Models;

class User
{
    protected int $id;
    protected string $name;
    protected string $email;

    public function __construct(int $id, string $name, string $email)
    {
        $this->id = $id;
        $this->name = $name;
        $this->email = $email;
    }

    public static function all(): array
    {
        return [
            new self(1, 'Alice Smith', 'alice@example.com'),
            new self(2, 'Bob Jones', 'bob@example.com'),
        ];
    }

    public function getId(): int
    {
        return $this->id;
    }

    public function getName(): string
    {
        return $this->name;
    }

    public function getEmail(): string
    {
        return $this->email;
    }
}
