<?php

namespace Tests\Feature;

use App\Http\Controllers\UserController;
use App\Services\UserService;

class UserTest
{
    public function testUserList(): void
    {
        $service = new UserService();
        $controller = new UserController($service);
        // Unit test assertion for fixture
    }
}
