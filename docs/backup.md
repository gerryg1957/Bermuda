
1. Back up the repository

Because Git already contains the complete history, the cleanest backup is a Git bundle:


git bundle verify ~/backups/moyodb/moyodb-$(date +%F).bundle
Enumerating objects: 1797, done.
Counting objects: 100% (1797/1797), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1499/1499), done.
Writing objects: 100% (1797/1797), 100.00 MiB | 15.93 MiB/s, done.
Total 1797 (delta 441), reused 726 (delta 149), pack-reused 0 (from 0)
gerry@stone-free:~/moyodb-core> git bundle verify ~/backups/moyodb/moyodb-$(date +%F).bundle
The bundle contains these 4 refs:
700f4c0c1d9eaecd02ef4b4c320000adec34224f refs/heads/master
16a6dfbb05e08e5c92a5591c2868fc255e6bd7d7 refs/tags/v0.1.0
49e9e2dc777240fb78b47280bcd3c1103bc6d9a5 refs/tags/v0.3.0
700f4c0c1d9eaecd02ef4b4c320000adec34224f HEAD
The bundle records a complete history.
The bundle uses this hash algorithm: sha1
/home/gerry/backups/moyodb/moyodb-core-2026-07-16.bundle is okay

This single file contains all commits, branches, and tags. You can restore it later with:

git clone ~/backups/moyodb/moyodb-core-2026-07-16.bundle moyodb-restored

You may also keep a readable compressed copy of the working directory:


gerry@stone-free:~/moyodb-core> tar --exclude='moyodb-core/target' \
>     -czf ~/backups/moyodb/moyodb-core-files-$(date +%F).tar.gz \
>     -C ~ moyodb-core

2. Back up the database

First make a consistent SQLite snapshot:

sqlite3 ~/go-database-go4go/metadata.sqlite3 \
  ".backup '$HOME/go-database-go4go/metadata-backup.sqlite3'"
  
  Then archive the whole database directory:
  
  tar -czf ~/backups/moyodb/go-database-go4go-$(date +%F).tar.gz \
    -C ~ go-database-go4go
    
    After the archive succeeds, remove the temporary SQLite copy from the live database:
    
    rm ~/go-database-go4go/metadata-backup.sqlite3
    
    
   3. Check the backups
   
   ls -lh ~/backups/moyodb
   
   Test the archives without extracting:
   
   tar -tzf ~/backups/moyodb/moyodb-core-files-$(date +%F).tar.gz | head
   tar -tzf ~/backups/moyodb/go-database-go4go-$(date +%F).tar.gz | head
   
