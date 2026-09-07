# Installer research boundary

No installer is implemented. Start R02 using a pinned osbuild/image-builder, a separate Anaconda installer environment, and the exact signed OS payload digest. Prove interactive disk choice, encryption, account/recovery setup, enrollment, and the next signed update. Never install automatically to the first discovered disk. UEFI Secure Boot and OCI signatures are separate tests. No default passwords or real keys belong here.
