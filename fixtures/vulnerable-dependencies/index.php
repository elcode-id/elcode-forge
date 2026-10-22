<?php

require_once __DIR__ . '/vendor/autoload.php';

use GuzzleHttp\Client;

$client = new Client();
$response = $client->request('GET', 'https://api.example.com');
