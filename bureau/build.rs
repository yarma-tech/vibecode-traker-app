fn main() {
    // Le dossier que l'empaquetage remplit doit exister des la compilation :
    // `tauri.conf.json` l'annonce en ressource, et une ressource absente
    // arrete la compilation. Vide, il est simplement ignore - c'est
    // `embarquer-le-service.sh` qui le remplit, juste avant l'empaquetage.
    // POURQUOI ici plutot qu'un fichier temoin versionne : le script efface et
    // refait ce dossier a chaque empaquetage, et emporterait le temoin avec.
    let _ = std::fs::create_dir_all("service-embarque");

    tauri_build::build()
}
