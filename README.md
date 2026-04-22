[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/b5MRUqco)

# SysWatch — Moniteur Système en Réseau

## Description
`SysWatch` est un serveur TCP interactif développé en **Rust** qui collecte et diffuse en temps réel les métriques système de la machine hôte. Il permet à plusieurs clients connectés sur le même réseau (WiFi/LAN) de surveiller l'utilisation du CPU, de la RAM et des processus, le tout sécurisé par un token d'authentification et journalisé automatiquement.

## ✨ Fonctionnalités
-  **Collecte en temps réel** : CPU (global & par cœur), RAM (utilisée/total/libre), Top 5 processus gourmands.
-  **Serveur TCP Multi-threadé** : Port `7878`, écoute sur toutes les interfaces (`0.0.0.0`), gestion concurrente des clients via `Arc<Mutex<T>>`.
-  **Authentification** : Accès protégé par le token `ENSPD2026`.
-  **Interface Texte** : Commandes interactives avec barres de progression ASCII.
-  **Journalisation** : Historique horodaté des connexions et commandes dans `syswatch.log`.
-  **Bonus P2P** : Commande `msg <IP> <texte>` pour envoyer des messages directs entre machines sur le réseau.

## 🛠️ Prérequis
- [Rust](https://www.rust-lang.org/) & [Cargo](https://doc.rust-lang.org/cargo/) (édition 2021)
- Un client TCP (`telnet`, `netcat` ou PowerShell) pour tester les connexions
- Accès réseau local (WiFi/Ethernet) pour les connexions distantes

##  Installation & Lancement

1. **Compiler le projet** :
   ```bash
   cargo build --release
