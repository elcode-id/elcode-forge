<?php

namespace App\Http\Controllers;

use App\Services\UserService;
use Illuminate\Http\Request;
use Illuminate\Http\JsonResponse;

class UserController
{
    protected UserService $userService;

    public function __construct(UserService $userService)
    {
        $this->userService = $userService;
    }

    public function index(Request $request): JsonResponse
    {
        $users = $this->userService->listUsers();
        return new JsonResponse(['data' => $users]);
    }

    public function show(int $id): JsonResponse
    {
        $user = $this->userService->findUser($id);
        return new JsonResponse(['data' => $user]);
    }
}
