mkdir modules
mkdir modules/base
cp /usr/lib/jvm/java-11-openjdk/jmods/java.base.jmod modules/base
cd modules/base
jmod extract java.base.jmod
cd ..
