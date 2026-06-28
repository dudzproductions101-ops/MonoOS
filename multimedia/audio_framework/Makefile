CXX      := g++
CXXFLAGS := -std=c++17 -O2 -Wall -Wextra -fPIC -pthread
LDFLAGS  := -shared -pthread

TARGET  := libmonoos_audio.so
SRCS    := audio_engine.cpp
OBJS    := $(SRCS:.cpp=.o)

all: $(TARGET)
$(TARGET): $(OBJS)
	$(CXX) $(LDFLAGS) -o $@ $^
%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@
clean:
	rm -f $(OBJS) $(TARGET)
