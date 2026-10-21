<?php

namespace App\Services;

use App\Models\User;
use App\Http\Controllers\UserController;
use GuzzleHttp\Client;

class UserService
{
    protected Client $httpClient;

    public function __construct()
    {
        $this->httpClient = new Client();
    }

    public function listUsers(): array
    {
        return User::all();
    }

    public function findUser(int $id): ?User
    {
        foreach (User::all() as $user) {
            if ($user->getId() === $id) {
                return $user;
            }
        }
        return null;
    }

    public function notifyController(UserController $controller): void
    {
        // Reference to UserController creates intentional cycle for testing
    }
}
